#!/usr/bin/env python3

grid = []
with open("/dev/stdin") as f:
    for line in f:
        line = line.strip()
        if line == "":
            continue

        line = [(1 if x == '@' else 0) for x in line]
        print(line)
        grid.append(line)

h = len(grid)
w = len(grid[0])
# print(grid)

neighbours = (
    (-1, -1), (-1, 0), (-1, 1),
    (0, -1), (0, 1),
    (1, -1), (1, 0), (1, 1)
)

sum = 0
for y in range(h):
    for x in range(w):
        if not grid[y][x]:
            continue
        
        around = 0
        for (dy, dx) in neighbours:
            py = y + dy
            px = x + dx
            if 0 <= py < h and 0 <= px < w and grid[py][px]:
                around += 1

        print(f"y{y}/x{x}: {around}")
        if around < 4:
            sum += 1



print(sum)
