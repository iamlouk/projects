#!/usr/bin/env python
import sys

input = [[int(c) for c in list(line.strip())] for line in sys.stdin]

ROWS = len(input)
COLS = len(input[0])

print(input)
print(f"rows={ROWS}, cols={COLS}")

offsets = [(-1, 0), (1, 0), (0, -1), (0, 1)]

def count_trailheads(x: int, y: int, lvl: int, visited: set) -> int:
    assert input[x][y] == lvl
    if lvl == 9:
        if (x, y) in visited:
            return 0
        else:
            visited.add((x, y))
            return 1

    sum = 0
    for off in offsets:
        nx, ny = x + off[0], y + off[1]
        if not(0 <= nx < ROWS) or not(0 <= ny < COLS):
            continue
        if input[nx][ny] != lvl + 1:
            continue

        sum += count_trailheads(nx, ny, lvl + 1, visited)

    return sum

total = 0
for x, row in enumerate(input):
    for y, n in enumerate(row):
        if n == 0:
            trailheads = count_trailheads(x, y, 0, set())
            print(f"x={x}, y={y}, trailheads={trailheads}")
            total += trailheads

print(total)
