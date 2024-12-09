#!/usr/bin/env python3
import sys, enum, itertools, math

input = [list(line.strip()) for line in sys.stdin]
ROWS, COLS = len(input), len(input[0])

class Freq:
    def __init__(self, id):
        self.id = id
        self.positions = []

    def __repr__(self):
        return f"<Freq:id={self.id}, positions={self.positions}>"

    def add_antenna(self, pos):
        self.positions.append(pos)

freqs: dict[str, Freq] = dict()
for row in range(0, ROWS):
    for col in range(0, COLS):
        c = input[row][col]
        if c == '.':
            continue

        freq = freqs.get(c)
        if freq is None:
            freq = Freq(c)
            freqs[c] = freq
        freq.add_antenna((row, col))

antinodes = dict()
for _, freq in freqs.items():
    for p1 in freq.positions:
        for p2 in freq.positions:
            if p1 == p2:
                continue;

            offset = (p1[0] - p2[0], p1[1] - p2[1])
            antinode = (p1[0] + offset[0], p1[1] + offset[1])
            if not(0 <= antinode[0] < ROWS) or not(0 <= antinode[1] < COLS):
                continue

            print(f"freq={freq.id}, p1={p1}, p2={p2}, antinode={antinode}")
            antinodes[antinode] = True

print(len(antinodes))
