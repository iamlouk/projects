#include <cstdio>
#include <cstdlib>

#include "./splay-tree.hh"

int main(int argc, char *argv[]) {
	printf("Hello World!\n");
	(void)argc;
	(void)argv;

	struct splay_tree<uint32_t, uint32_t> stree;

	stree.insert(5, 5);
	stree.insert(1, 1);
	stree.insert(3, 3);
	stree.insert(7, 7);
	stree.insert(8, 8);
	stree.insert(9, 9);
	stree.insert(0, 0);
	stree.insert(10, 10);
	stree.insert(11, 11);

	int d = 0;
	uint32_t *res = stree.lookup(5, &d);
	assert(d == 0 && *res == 5);

	res = stree.lookup(8, &d);
	assert(d > 0 && *res == 8);

	res = stree.lookup(8, &d);
	assert(d == 0 && *res == 8);

	res = stree.lookup(1, &d);
	assert(d > 0 && *res == 1);

	res = stree.lookup(0, &d);
	assert(d == 1 && *res == 0);

	return EXIT_SUCCESS;
}

