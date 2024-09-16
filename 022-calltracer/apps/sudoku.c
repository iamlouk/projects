#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <stdbool.h>
#include <assert.h>
#include <math.h>
#include <string.h>

typedef uint8_t digit_t;

struct bitset { uint64_t bits; size_t size; };

static bool bitset_contains(struct bitset *set, digit_t bit) {
	assert(bit < sizeof(uint64_t) * 8);
	return (set->bits & ((uint64_t)1 << (uint64_t)bit)) != 0;
}

static bool bitset_set(struct bitset *set, digit_t bit) {
	bool res = bitset_contains(set, bit);
	set->size += res ? 0 : 1;
	set->bits |= ((uint64_t)1 << (uint64_t)bit);
	return res;
}

static bool bitset_unset(struct bitset *set, digit_t bit) {
	bool res = bitset_contains(set, bit);
	set->size -= res ? 1 : 0;
	set->bits &= ~((uint64_t)1 << (uint64_t)bit);
	return res;
}

static void bitset_reset(struct bitset *set, digit_t start, digit_t end) {
	set->bits = 0;
	for (size_t i = start; i < end; i++)
		bitset_set(set, i);
	set->size = end - start;
}

static bool bitset_empty(struct bitset *set) { return set->bits == 0; }

static digit_t bitset_first(struct bitset *set, digit_t start, digit_t end) {
	for (digit_t i = start; i < end; i++)
		if (bitset_contains(set, i))
			return i;

	assert(false);
}

#define NUMBERS 9
#define SQRTNUMBERS 3

static digit_t game[NUMBERS][NUMBERS] = {
#if 0
	{1, 0, 0, 0, 0, 0, 0, 0, 0},
	{2, 0, 0, 0, 0, 0, 0, 0, 0},
	{3, 0, 0, 0, 0, 0, 0, 0, 0},
	{4, 0, 0, 0, 0, 0, 0, 0, 0},
	{5, 0, 0, 0, 0, 0, 0, 0, 0},
	{6, 0, 0, 0, 0, 0, 0, 0, 0},
	{7, 0, 0, 0, 0, 0, 0, 0, 0},
	{8, 1, 2, 3, 4, 5, 6, 7, 0},
	{0, 0, 0, 0, 0, 0, 0, 0, 0}

#endif
#if 0
	{0, 1, 8, 0, 0, 2, 3, 0, 4},
	{0, 0, 3, 5, 0, 0, 0, 0, 0},
	{5, 2, 4, 8, 9, 0, 0, 0, 0},
	{1, 0, 5, 0, 7, 0, 4, 0, 6},
	{0, 0, 7, 0, 0, 0, 9, 0, 0},
	{2, 0, 9, 0, 4, 0, 5, 0, 8},
	{0, 0, 0, 0, 8, 9, 6, 4, 3},
	{0, 0, 0, 0, 0, 7, 2, 0, 0},
	{3, 0, 1, 6, 0, 0, 7, 8, 0}
#endif
#if 1
	{0, 0, 0, 8, 0, 0, 0, 0, 9},
	{0, 1, 9, 0, 0, 5, 8, 3, 0},
	{0, 4, 3, 0, 1, 0, 0, 0, 7},
	{4, 0, 0, 1, 5, 0, 0, 0, 3},
	{0, 0, 2, 7, 0, 4, 0, 1, 0},
	{0, 8, 0, 0, 9, 0, 6, 0, 0},
	{0, 7, 0, 0, 0, 6, 3, 0, 0},
	{0, 3, 0, 0, 7, 0, 0, 8, 0},
	{9, 0, 4, 5, 0, 0, 0, 0, 1}
#endif
};

struct pos_t { size_t row, col; };
static struct pos_t groups[NUMBERS][NUMBERS][3][NUMBERS];
static struct bitset possibilities[NUMBERS][NUMBERS];
enum solver_state { SOLVER_DONE, SOLVER_WRONG, SOLVER_STUCK };

static void print_game(FILE *f, const uint8_t game[NUMBERS][NUMBERS]) {
	for (size_t i = 0; i < NUMBERS; i++) {
		if (i % SQRTNUMBERS == 0 && i != 0) {
			for (size_t j = 0; j < NUMBERS; j++)
				fprintf(f, j % SQRTNUMBERS == 0 && j != 0 ? "+---" : "---");
			fprintf(f, "\n");
		}

		for (size_t j = 0; j < NUMBERS; j++) {
			const char *sep = j % SQRTNUMBERS == 0 && j != 0 ? "|" : "";
			if (game[i][j])
				fprintf(f, "%s %X ", sep, game[i][j]);
			else
				fprintf(f, "%s   ", sep);
		}
		fprintf(f, "\n");
	}
}

static enum solver_state eliminate_possibilities(digit_t game[NUMBERS][NUMBERS], struct bitset possibilities[NUMBERS][NUMBERS]) {
	bool done;
	bool change;
	size_t iters = 0;
	do {
		done = true;
		change = false;
		for (size_t row = 0; row < NUMBERS; row++) {
			for (size_t col = 0; col < NUMBERS; col++) {
				struct bitset *set = &possibilities[row][col];
				if (bitset_empty(set))
					return SOLVER_WRONG;

				if (game[row][col] != 0)
					continue;

				for (size_t gi = 0; gi < 3; gi++) {
					struct pos_t *group = groups[row][col][gi];
					for (size_t i = 0; i < NUMBERS; i++) {
						digit_t digit = game[group[i].row][group[i].col];
						if (digit == 0)
							continue;

						bitset_unset(set, digit);
					}
				}

				if (set->size == 1) {
					change = true;
					digit_t d = bitset_first(set, 1, NUMBERS+1);
					game[row][col] = d;
					printf("elimination: game[%ld][%ld] = %X (iter: %ld)\n", row, col, d, iters);
					// print_game(stdout, game);
					continue;
				}
				done = false;
				continue;
			}
		}
		iters += 1;
	} while (change && !done);
	return done ? SOLVER_DONE : SOLVER_STUCK;
}

struct solver_option { size_t row; size_t col; size_t choices; };
static int prioritize(const void *vo1, const void *vo2) {
	const struct solver_option *o1 = vo1;
	const struct solver_option *o2 = vo2;
	return o1->choices > o2->choices;
}

static enum solver_state solve(digit_t game[NUMBERS][NUMBERS], struct bitset possibilities[NUMBERS][NUMBERS]) {
	enum solver_state state = eliminate_possibilities(game, possibilities);
	if (state != SOLVER_STUCK) {
		printf("state: %s\n", state == SOLVER_DONE ? "done" : "wrong");
		return state;
	}

	size_t num_options = 0;
	struct solver_option options[NUMBERS*NUMBERS];
	for (size_t row = 0; row < NUMBERS; row++) {
		for (size_t col = 0; col < NUMBERS; col++) {
			if (game[row][col] != 0)
				continue;

			assert(possibilities[row][col].size <= NUMBERS);
			options[num_options++] = (struct solver_option){
				.row = row, .col = col,
				.choices = possibilities[row][col].size
			};
		}
	}

	qsort(&options[0], num_options, sizeof(struct solver_option), prioritize);
	for (size_t i = 0; i < num_options; i++) {
		struct solver_option *option = &options[i];
		size_t try = 0;
		struct bitset *set = &possibilities[option->row][option->col];
		for (digit_t d = 1; d <= NUMBERS; d++) {
			if (!bitset_contains(set, d))
				continue;

			printf("guess: game[%ld][%ld] = %X (choices: %ld, try: %ld)\n",
					option->row, option->col, d, option->choices, ++try);
			
			digit_t old_game[NUMBERS][NUMBERS];
			struct bitset old_possibilities[NUMBERS][NUMBERS];
			memcpy(old_game, game, sizeof(old_game));
			memcpy(old_possibilities, possibilities, sizeof(old_possibilities));

			game[option->row][option->col] = d;
			print_game(stdout, game);
			state = solve(game, possibilities);
			if (state == SOLVER_DONE)
				return SOLVER_DONE;

			memcpy(game, old_game, sizeof(old_game));
			memcpy(possibilities, old_possibilities, sizeof(old_possibilities));
		}
	}
	return SOLVER_WRONG;
}

int main(int argc, const char *argv[]) {
	(void) argc;
	(void) argv;
	assert(sqrt(NUMBERS) == SQRTNUMBERS && SQRTNUMBERS * SQRTNUMBERS == NUMBERS);
	for (size_t row = 0; row < NUMBERS; row++) {
		for (size_t col = 0; col < NUMBERS; col++) {
			for (size_t i = 0; i < NUMBERS; i++) {
				groups[row][col][0][i] = (struct pos_t){ row, i };
				groups[row][col][1][i] = (struct pos_t){ i, col };

				size_t grow = row / 3, gcol = col / 3;
				groups[row][col][2][i] = (struct pos_t){
					grow * 3 + i % SQRTNUMBERS,
	 				gcol * 3 + i / SQRTNUMBERS
				};
			}
		}
	}
	for (size_t i = 0; i < NUMBERS; i++)
		for (size_t j = 0; j < NUMBERS; j++)
			bitset_reset(&possibilities[i][j], 1, NUMBERS+1);

	print_game(stdout, game);
	solve(game, possibilities);
	print_game(stdout, game);

	return EXIT_SUCCESS;
}

