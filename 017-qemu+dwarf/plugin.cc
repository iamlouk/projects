#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <cstdint>
#include <cassert>
#include <utility>
#include <vector>
#include <functional>
#include <algorithm>
#include <map>
#include <unordered_map>

#include "./libelfin/elf/elf++.hh"
#include "./libelfin/dwarf/dwarf++.hh"

#include "utils.hh"

extern "C" {
	#include "qemu/qemu-plugin.h"
	#include <fcntl.h>
	QEMU_PLUGIN_EXPORT int qemu_plugin_version = QEMU_PLUGIN_VERSION;
}

static const size_t top_n = 10;
static uint64_t exit_address = 0x0;
static elf::elf *elf_binary = nullptr;
static dwarf::dwarf *dwarf_binary = nullptr;
static std::unordered_map<uint64_t, struct translation_block*> *tbs = nullptr;

struct translation_block {
	size_t ninsns;
	uint64_t exec_count;
	struct address_range addr;

	const char *source_file;
	struct address_range source_lines;
};

static bool plugin_exit_executed = false;
static void plugin_exit(qemu_plugin_id_t id, void *arg) {
	(void) id;
	(void) arg;
	if (plugin_exit_executed) return;
	plugin_exit_executed = true;

	const char *prefix = "QEMU:";
	fprintf(stderr, "%s total TBs executed %ld, top:\n", prefix, tbs->size());

	/* all of this would be much more performant with sorted inserts and so on: */
	std::vector<struct translation_block*> merging_tbs;
	merging_tbs.reserve(tbs->size());
	for (std::pair<uint64_t, struct translation_block*> entry: *tbs)
		merging_tbs.push_back(entry.second);
	std::sort(merging_tbs.begin(), merging_tbs.end(), [](
			struct translation_block *a,
			struct translation_block *b) {
		assert(a->addr.first != b->addr.first);
		return a->addr.last < b->addr.last;
	});

	/* merge tbs that cover the same instructions */
	std::vector<struct translation_block*> merged_tbs;
	merged_tbs.reserve(tbs->size());
	for (size_t i = 0; i < merging_tbs.size(); i++) {
		struct translation_block *tb = merging_tbs[i];
		for (size_t j = i + 1; j < merging_tbs.size(); j++) {
			struct translation_block *otb = merging_tbs[j];
			if (otb->addr.last != tb->addr.last)
				break;

			tb->exec_count += otb->exec_count;
			tb->source_lines.first = std::min(tb->source_lines.first, otb->source_lines.first);
			tb->source_lines.last = std::max(tb->source_lines.last, otb->source_lines.last);
			i += 1;
		}
		merged_tbs.push_back(tb);
	}

	/* a top k algorithm would be much more performant: */
	std::sort(merged_tbs.begin(), merged_tbs.end(), [](
			struct translation_block *a,
			struct translation_block *b) {
		return a->exec_count > b->exec_count;
	});

	for (size_t i = 0; i < top_n; i++) {
		struct translation_block *tb = merged_tbs[i];
		fprintf(stderr, "%s [\t%ld] -> %s:%ld-%ld was executed %ld times\n",
				prefix, i + 1, tb->source_file, tb->source_lines.first, tb->source_lines.last, tb->exec_count);
	}

	for (std::pair<uint64_t, struct translation_block *> entry: *tbs)
		free(entry.second);
	delete tbs;
	delete elf_binary;
	delete dwarf_binary;
}

static void vcpu_tb_trans(qemu_plugin_id_t id, struct qemu_plugin_tb *qemu_tb) {
	(void) id;
	if (plugin_exit_executed)
		return;

	struct address_range addr;
	size_t ninsns = qemu_plugin_tb_n_insns(qemu_tb);
	addr.first = qemu_plugin_tb_vaddr(qemu_tb);
	addr.last = qemu_plugin_insn_vaddr(qemu_plugin_tb_get_insn(qemu_tb, ninsns - 1));
	if (addr.first == exit_address) {
		fprintf(stderr, "QEMU: exit_address reached\n");
		plugin_exit(0, NULL);
		return;
	}

	struct translation_block *tb = nullptr;
	auto iter = tbs->find(addr.first);
	if (iter != tbs->end()) {
		assert(iter->second->ninsns == ninsns && iter->second->addr.last == addr.last);
		return;
	}

	tb = (struct translation_block *)malloc(sizeof(struct translation_block));
	tb->exec_count = 0;
	tb->addr = addr;
	tb->ninsns = ninsns;
	tb->source_file = nullptr;
	tb->source_lines.first = 0;
	tb->source_lines.last = 0;
	for (const dwarf::compilation_unit &comp_unit: dwarf_binary->compilation_units()) {
		assert(comp_unit.valid());
		const dwarf::line_table &line_table = comp_unit.get_line_table();
		const dwarf::line_table::iterator first_line = line_table.find_address(addr.first);
		if (first_line == line_table.end())
			break;

		const dwarf::line_table::iterator last_line = line_table.find_address(addr.last);
		if (last_line == line_table.end())
			break;

		tb->source_file = first_line->file->path.c_str();
		tb->source_lines.first = first_line->line;
		tb->source_lines.last = last_line->line;
	}

	if (!tb->source_file)
		return;

	(*tbs)[addr.first] = tb;
	qemu_plugin_register_vcpu_tb_exec_inline(
			qemu_tb, QEMU_PLUGIN_INLINE_ADD_U64, (void *)&tb->exec_count, 1);
}

extern "C" QEMU_PLUGIN_EXPORT
int qemu_plugin_install(qemu_plugin_id_t id, const qemu_info_t *info,
                        int argc, char **argv) {
	(void) info;
	const char *binary = getenv("QEMU_EXEC_BINARY"); /*qemu_plugin_path_to_binary();*/

	exit_address = strtoll(getenv("QEMU_EXIT_ADDRESS"), NULL, 16);

	fprintf(stderr, "QEMU plugin loaded: argc=%d, argv=[", argc);
	for (int i = 0; i < argc; i++)
		fprintf(stderr, i == 0 ? "'%s'" : ", '%s'", argv[i]);
	fprintf(stderr, "], binary='%s', exit_address=%#lx\n", binary, exit_address);

	int fd = open(binary, O_RDONLY);
	elf_binary = new elf::elf(elf::create_mmap_loader(fd));
	dwarf_binary = new dwarf::dwarf(dwarf::elf::create_loader(*elf_binary));
	tbs = new std::unordered_map<uint64_t, struct translation_block*>();

	qemu_plugin_register_vcpu_tb_trans_cb(id, vcpu_tb_trans);
	qemu_plugin_register_atexit_cb(id, plugin_exit, NULL);
	return 0;
}

