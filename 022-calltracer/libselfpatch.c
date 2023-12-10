#include <assert.h>
#include <signal.h>
#include <stdatomic.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <sys/mman.h>
#include <elf.h>

static const size_t PAGE_SIZE = 4096;

static bool debug = true;

static struct patched {
	Elf64_Sym elf_sym;
	uintptr_t start;
	_Atomic uint64_t stats[4];
	struct patched *next;
	char name[];
} *patched_head = NULL;

// Might execute in parallel:
__attribute__((weak))
extern void libselfpatch_interceptor(struct patched *called) {
	if (debug)
		fprintf(stderr, "intercepted: %s (%p <-> %lx)!\n",
				called->name, __builtin_return_address(0), called->start);
	atomic_fetch_add(&called->stats[0], 1);
}

static int match(const char *regex, const char *text);

static bool libselfpatch_should_patch(const Elf64_Sym *sym, const char *name) {
	(void) sym;
	static char *patterns = NULL;
	static size_t num_patterns = 0;
	if (!patterns) {
		patterns = getenv("LSP_TO_PATCH");
		if (!patterns) {
			fprintf(stderr, "LSP: getenv(\"LSP_TO_PATCH\") is NULL\n");
			return false;
		}

		char *saveptr = patterns;
		while (strtok_r(saveptr, "|", &saveptr))
			num_patterns += 1;

		char *pattern = patterns;
		for (size_t i = 0; i < num_patterns; i++, pattern += strlen(pattern)+1) {
			fprintf(stderr, "LSP: patterns[%ld]: '%s'\n", i, pattern);
		}
	}

	char *pattern = patterns;
	for (size_t i = 0; i < num_patterns; i++, pattern += strlen(pattern)+1)
		if (match(pattern, name))
			return true;

	return false;
}

static int libselfpatch_make_writeable(uintptr_t start, size_t len);
static int libselfpatch_restore_permissions();

static int libselfpatch_patch_me(const Elf64_Sym *sym, const char *name,
		const Elf64_Shdr *section, const uint8_t *code, size_t size) {
	(void) sym;
	(void) section;

	// Check for some NOPs at the start of the function...
	for (size_t i = 0; i < 32; i++)
		if (code[i] != 0x90)
			return fprintf(stderr, "LSP: %s: missing NOPs (no -fpatchable-function-entry=32?)\n",
					name);

	if (libselfpatch_make_writeable((uintptr_t)code, size))
		return EXIT_FAILURE;

	struct patched *patched = calloc(1, sizeof(struct patched) + strlen(name) + 1);
	patched->elf_sym = *sym;
	patched->start = (uintptr_t)code;
	patched->next = patched_head;
	strcpy(patched->name, name);
	patched_head = patched;

	uint64_t iarg = (uint64_t)patched;
	uint64_t ifn = (uint64_t)&libselfpatch_interceptor;
	const uint8_t patch[] = {
		// 0x50, /* push %rax */
		0x57, /* push %rdi */
		0x56, /* push %rsi */
		0x52, /* push %rdx */
		0x51, /* push %rcx */
		0x41, 0x50, /* push %r8 */
		0x41, 0x51, /* push %r9 */
		0x48, 0xbf, /* movabs $<name...>, %rdi */
		(iarg >>  0) & 0xff, (iarg >>  8) & 0xff,
		(iarg >> 16) & 0xff, (iarg >> 24) & 0xff,
		(iarg >> 32) & 0xff, (iarg >> 40) & 0xff,
		(iarg >> 48) & 0xff, (iarg >> 56) & 0xff,
		0x48, 0xb8, /* movabs $<fn...>, %rax */
		(ifn >>  0) & 0xff, (ifn >>  8) & 0xff,
		(ifn >> 16) & 0xff, (ifn >> 24) & 0xff,
		(ifn >> 32) & 0xff, (ifn >> 40) & 0xff,
		(ifn >> 48) & 0xff, (ifn >> 56) & 0xff,
		0xff, 0xd0, /* call *%rax */
		0x41, 0x59, /* pop %r9 */
		0x41, 0x58, /* pop %r8 */
		0x59, /* pop %rcx */
		0x5a, /* pop %rdx */
		0x5e, /* pop %rsi */
		0x5f, /* pop %rdi */
		// 0x58  /* pop %rax */
	};

	assert(sizeof(patch) <= 42);
	for (size_t i = 0; i < sizeof(patch); i++)
		((uint8_t*)code)[i] = patch[i];
	for (size_t i = 0; i < sizeof(patch); i += 32)
		asm("clflush (%0)" :: "r"(&code[i]));

	return EXIT_SUCCESS;
}

static const char *binary = NULL;
static size_t binary_size = 0;

__attribute__((constructor))
static int libselfpatch_init() {
	char self_path[128];
	snprintf(self_path, sizeof(self_path), "/proc/%d/exe", getpid());
	int fd = open(self_path, O_RDONLY);
	if (fd == -1)
		return fprintf(stderr, "LSP: failed to open %s: %s\n", self_path, strerror(errno));

	struct stat stat = {0};
	if (fstat(fd, &stat) == -1)
		return fprintf(stderr, "LSP: stat failed on %s: %s\n", self_path, strerror(errno));

	binary_size = stat.st_size;
	binary = mmap(NULL, stat.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
	if (!binary || (intptr_t)binary == -1)
		return fprintf(stderr, "LSP: mmap failed for %s: %s\n", self_path, strerror(errno));

	close(fd); // The mapping stays valid.
	const Elf64_Ehdr *header = (const Elf64_Ehdr *)&binary[0];
	if (header->e_ident[EI_MAG0] != ELFMAG0
		|| header->e_ident[EI_MAG1] != ELFMAG1
		|| header->e_ident[EI_MAG2] != ELFMAG2
		|| header->e_ident[EI_MAG3] != ELFMAG3
		|| header->e_ident[EI_CLASS] != ELFCLASS64)
		return fprintf(stderr, "LSP: not a ELF file: %s\n", self_path);

	if (header->e_machine != EM_X86_64)
		return fprintf(stderr, "LSP: machine architecture is not amd64\n");

	const Elf64_Shdr *sh_tbl = (const Elf64_Shdr *)&binary[header->e_shoff];
	const Elf64_Shdr *sh_snames = &sh_tbl[header->e_shstrndx];
	assert(sh_snames->sh_type == SHT_STRTAB);
	const char *section_names = (const char *)&binary[sh_snames->sh_offset];
	for (size_t i = 0; i < header->e_shnum; i++) {
		const Elf64_Shdr *sh = &sh_tbl[i];
		if (sh->sh_type != SHT_SYMTAB || !sh->sh_link)
			continue;

		const Elf64_Shdr *strtab_header = &sh_tbl[sh->sh_link];
		const char *strtab = (const char *)&binary[strtab_header->sh_offset];
		const Elf64_Sym *symtab = (const Elf64_Sym *)&binary[sh->sh_offset];

		size_t num_entries = sh->sh_size / sh->sh_entsize;
		assert(num_entries * sh->sh_entsize == sh->sh_size && sh->sh_entsize == sizeof(Elf64_Sym));
		for (size_t i = 0; i < num_entries; i++) {
			const Elf64_Sym *sym = &symtab[i];
			if (ELF64_ST_TYPE(sym->st_info) != STT_FUNC || sym->st_name == 0
					|| sym->st_value == 0 || sym->st_size <= 16)
				continue;

			const char *name = &strtab[sym->st_name];
			const Elf64_Shdr *section = &sh_tbl[sym->st_shndx];
			if (section->sh_type != SHT_PROGBITS
					|| !(section->sh_flags & (SHF_EXECINSTR | SHF_ALLOC))
					|| !libselfpatch_should_patch(sym, name)
					|| strncmp("libselfpatch_", name, strlen("libselfpatch_")) == 0)
				continue;

			const char *section_name = &section_names[section->sh_name];
			if (debug)
				fprintf(stderr, "LSP: patching <%s> in '%s': %#08lx (size: %ld)\n",
						name, section_name, sym->st_value, sym->st_size);

			const uint8_t *code = (const uint8_t *)(sym->st_value);
			libselfpatch_patch_me(sym, name, section, code, sym->st_size);
		}
	}

	libselfpatch_restore_permissions();
	fprintf(stderr, "LSP: init done!\n");
	return EXIT_SUCCESS;
}

__attribute__((destructor))
static int libselfpatch_fini() {
	munmap((void*)binary, binary_size);

	struct patched *patched = patched_head;
	while (patched) {
		struct patched *next = patched->next;
		if (debug)
			fprintf(stderr, "LSP: <%s> was called %ld times!\n", patched->name, patched->stats[0]);
		free(patched);
		patched = next;
	}

	return EXIT_SUCCESS;
}

static struct unprotected_page_cache {
	uintptr_t page;
	struct unprotected_page_cache *next;
} *unprotected_pages = NULL;

static int libselfpatch_make_writeable(uintptr_t start, size_t len) {
	static uintptr_t PAGE_SIZE = 0;
	if (PAGE_SIZE == 0)
		PAGE_SIZE = sysconf(_SC_PAGESIZE);

	uintptr_t start_page = start & ~(PAGE_SIZE - 1);
	uintptr_t end_page = (start + len) & ~(PAGE_SIZE - 1);
	assert(start_page == (start - (start % PAGE_SIZE)));

	bool start_page_found = false, end_page_found = false;
	for (struct unprotected_page_cache *pc = unprotected_pages;
			pc != NULL && !(start_page_found && end_page_found); pc = pc->next) {
		start_page_found |= pc->page == start_page;
		end_page_found |= pc->page == end_page;
	}

	if (!start_page_found) {
		if (mprotect((void *)start_page, PAGE_SIZE, PROT_READ | PROT_WRITE | PROT_EXEC))
			return fprintf(stderr, "LSP: mprotect(%p, %ld, ...) failed: %s\n",
				(void*)start_page, PAGE_SIZE, strerror(errno));
	
		struct unprotected_page_cache *page = malloc(sizeof(struct unprotected_page_cache));
		page->page = start_page;
		page->next = unprotected_pages;
		unprotected_pages = page;
	}

	if (!end_page_found && start_page != end_page) {
		if (mprotect((void *)end_page, PAGE_SIZE, PROT_READ | PROT_WRITE | PROT_EXEC))
			return fprintf(stderr, "LSP: mprotect(%p, %ld, ...) failed: %s\n",
				(void*)end_page, PAGE_SIZE, strerror(errno));

		struct unprotected_page_cache *page = malloc(sizeof(struct unprotected_page_cache));
		page->page = end_page;
		page->next = unprotected_pages;
		unprotected_pages = page;
	}

	return 0;
}

static int libselfpatch_restore_permissions() {
	struct unprotected_page_cache *pc = unprotected_pages;
	while (pc) {
		struct unprotected_page_cache *next = pc->next;
		if (mprotect((void*)pc->page, PAGE_SIZE, PROT_READ | PROT_EXEC))
			return fprintf(stderr, "LSP: mprotect(%p, %ld, ...) failed: %s\n",
				(void*)pc->page, PAGE_SIZE, strerror(errno));

		free(pc);
		pc = next;
	}
	unprotected_pages = NULL;
	return 0;
}

/* The following three functions were taken from:
 * https://www.cs.princeton.edu/courses/archive/spr09/cos333/beautiful.html
 */
static int match(const char *regexp, const char *text);
static int matchhere(const char *regexp, const char *text);
static int matchstar(int c, const char *regexp, const char *text);

/* match: search for regexp anywhere in text */
static int match(const char *regexp, const char *text) {
	if (regexp[0] == '^')
		return matchhere(regexp+1, text);
	do {    /* must look even if string is empty */
		if (matchhere(regexp, text))
			return 1;
	} while (*text++ != '\0');
	return 0;
}

/* matchhere: search for regexp at beginning of text */
static int matchhere(const char *regexp, const char *text) {
	if (regexp[0] == '\0')
		return 1;
	if (regexp[1] == '*')
		return matchstar(regexp[0], regexp+2, text);
	if (regexp[0] == '$' && regexp[1] == '\0')
		return *text == '\0';
	if (*text!='\0' && (regexp[0]=='.' || regexp[0]==*text))
		return matchhere(regexp+1, text+1);
	return 0;
}

/* matchstar: search for c*regexp at beginning of text */
static int matchstar(int c, const char *regexp, const char *text) {
	do {    /* a * matches zero or more instances */
		if (matchhere(regexp, text))
			return 1;
	} while (*text != '\0' && (*text++ == c || c == '.'));
	return 0;
}

