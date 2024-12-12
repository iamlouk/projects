#!/usr/bin/env python
import sys

map = [[(c, False) for c in line.strip()] for line in sys.stdin]
ROWS, COLS = len(map), len(map[0])
# print(f"rows={ROWS},cols={COLS},input={map}")

def plot(x: int, y: int):
    plant = map[x][y][0]
    print(f"start! ({x}, {y}), plant='{plant}'")
    worklist = [(x, y)]
    area, perimeter = 0, 0
    while len(worklist) > 0:
        x, y = worklist.pop()
        if map[x][y] != (plant, False):
            continue
        # print(f"next: ({x}, {y})")
        area += 1
        map[x][y] = (plant, True)
        for dx, dy in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
            nx, ny = x + dx, y + dy
            if not(0 <= nx < ROWS) or not(0 <= ny < COLS):
                perimeter += 1
                continue
            if map[nx][ny][0] != plant:
                perimeter += 1
            worklist.append((nx, ny))
    print(f"area={area},perimeter={perimeter}")
    return (area, perimeter)

res = 0
for x in range(ROWS):
    for y in range(COLS):
        if not map[x][y][1]:
            area, perimeter = plot(x, y)
            res += area * perimeter

print(f"res={res}")



