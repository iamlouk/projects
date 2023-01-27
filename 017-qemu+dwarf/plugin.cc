#include <cstdio>
#include <cstdlib>
#include <cstdint>
#include <cassert>
#include <elf++.hh>
#include <dwarf++.hh>

extern "C" {
	#include <qemu/qemu-plugin.h>
	#include <fcntl.h>
	QEMU_PLUGIN_EXPORT int qemu_plugin_version = QEMU_PLUGIN_VERSION;
}

static elf::elf *elf_binary = nullptr;
static dwarf::dwarf *dwarf_binary = nullptr;

struct translation_block {
	uint64_t exec_count;
};

static void vcpu_tb_exec(unsigned int cpu_index, void *arg) {
	(void) cpu_index;
	struct translation_block *tb = (struct translation_block*)arg;
}

static void plugin_exit(qemu_plugin_id_t id, void *arg) {
	(void) id;
	(void) arg;
	delete elf_binary;
	delete dwarf_binary;
}

static void vcpu_tb_trans(qemu_plugin_id_t id, struct qemu_plugin_tb *qemu_tb) {
	(void) id;

	struct translation_block *tb = (struct translation_block *)
		malloc(sizeof(struct translation_block));
	tb->exec_count = 0;

	qemu_plugin_register_vcpu_tb_exec_cb(qemu_tb,
			vcpu_tb_exec, QEMU_PLUGIN_CB_NO_REGS, (void *)tb);
}

extern "C" QEMU_PLUGIN_EXPORT
int qemu_plugin_install(qemu_plugin_id_t id, const qemu_info_t *info,
                        int argc, char **argv) {
	(void) info;
	const char *binary = "./example/hello-world.riscv"; /*qemu_plugin_path_to_binary();*/

	fprintf(stderr, "QEMU plugin loaded: argc=%d, argv=[", argc);
	for (int i = 0; i < argc; i++)
		fprintf(stderr, i == 0 ? "'%s'" : ", '%s'", argv[i]);
	fprintf(stderr, "], binary='%s'\n", binary);

	int fd = open(binary, O_RDONLY);

	elf_binary = new elf::elf(elf::create_mmap_loader(fd));
	dwarf_binary = new dwarf::dwarf(dwarf::elf::create_loader(*elf_binary));

	qemu_plugin_register_vcpu_tb_trans_cb(id, vcpu_tb_trans);
	qemu_plugin_register_atexit_cb(id, plugin_exit, NULL);
	return 0;
}

