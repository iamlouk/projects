#include <stddef.h>
#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>

size_t bruteforce(int64_t time, int64_t min_distance) {
	size_t winning = 0;
	#pragma omp parallel for reduction(+:winning)
	for (int64_t pressing = 1; pressing < time; pressing += 1) {
		int64_t speed = pressing;
		int64_t duration = (time - pressing);
		int64_t distance = speed * duration;
		if (distance > min_distance)
			winning += 1;
	}

	return winning;
}

int main() {
#if 0
	const size_t num_races = 4;
	int64_t durations[num_races];
	int64_t distances[num_races];

	if (fscanf(stdin, "Time: %ld %ld %ld %ld\n",
			   &durations[0], &durations[1],
			   &durations[2], &durations[3]) != num_races)
		return EXIT_FAILURE;
	if (fscanf(stdin, "Distance: %ld %ld %ld %ld\n",
			   &distances[0], &distances[1],
			   &distances[2], &distances[3]) != num_races)
		return EXIT_FAILURE;
#else
	const size_t num_races = 1;
	int64_t durations[num_races];
	int64_t distances[num_races];

	if (fscanf(stdin, "Time: %ld\n", &durations[0]) != num_races)
		return EXIT_FAILURE;
	if (fscanf(stdin, "Distance: %ld\n", &distances[0]) != num_races)
		return EXIT_FAILURE;
#endif

	size_t total = 1;
	for (size_t i = 0; i < num_races; i++) {
		fprintf(stderr, "Race #%ld: duration: %ld, distance: %ld",
				i + 1, durations[i], distances[i]);
		fflush(stderr);

		size_t waystowin = bruteforce(durations[i], distances[i]);
		fprintf(stderr, " -> %ld ways!\n", waystowin);
		total *= waystowin;
	}

	printf("result: %ld\n", total);
	return EXIT_SUCCESS;
}

