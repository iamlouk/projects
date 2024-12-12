#!/usr/bin/env python
import sys, math

map = [[(c, False) for c in line.strip()] for line in sys.stdin]
ROWS, COLS = len(map), len(map[0])
# print(f"rows={ROWS},cols={COLS},input={map}")

def is_new_side(x, y, dx, dy, plant, plants, sidesset) -> bool:
    isnew = ((x, y), (dx, dy)) not in sidesset
    # print(f"({x}, {y}), d=({dx}, {dy}) -> isnew={isnew}")
    if (dx, dy) == (-1, 0) or (dx, dy) == (1, 0):
        # check left/right side!
        sidesset.add(((x, y-1), (dx, dy)))
        sidesset.add(((x, y+0), (dx, dy)))
        sidesset.add(((x, y+1), (dx, dy)))
        pass
    elif (dx, dy) == (0, -1) or (dx, dy) == (0, 1):
        # check up/below side!
        sidesset.add(((x-1, y), (dx, dy)))
        sidesset.add(((x+0, y), (dx, dy)))
        sidesset.add(((x+1, y), (dx, dy)))
        pass
    else:
        assert False
    return isnew

def plot(x: int, y: int):
    plant = map[x][y][0]
    print(f"start! ({x}, {y}), plant='{plant}'")
    worklist = [(x, y)]
    area, perimeter, sides = 0, 0, 0
    plants = set()
    minx, miny, maxx, maxy = ROWS, COLS, 0, 0
    while len(worklist) > 0:
        x, y = worklist.pop()
        if map[x][y] != (plant, False):
            continue
        # print(f"next: ({x}, {y})")
        area += 1
        map[x][y] = (plant, True)
        plants.add((x, y))
        minx, miny, maxx, maxy = min(minx, x), min(miny, y), max(maxx, x), max(maxy, y)
        for dx, dy in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
            nx, ny = x + dx, y + dy
            if not(0 <= nx < ROWS) or not(0 <= ny < COLS):
                perimeter += 1
                continue
            if map[nx][ny][0] != plant:
                perimeter += 1
            worklist.append((nx, ny))

    sidesset = set()
    for x in range(minx, maxx + 1):
        for y in range(miny, maxy + 1):
            if (x, y) not in plants:
                continue
            for dx, dy in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
                nx, ny = x + dx, y + dy
                if 0 <= nx < ROWS and 0 <= ny < COLS and map[nx][ny][0] == plant:
                    continue
                if is_new_side(x, y, dx, dy, plant, plants, sidesset):
                    sides += 1

    print(f"area={area},perimeter={perimeter},sides={sides}")
    return (area, perimeter, sides)

res_part1, res_part2 = 0, 0
for x in range(ROWS):
    for y in range(COLS):
        if not map[x][y][1]:
            area, perimeter, sides = plot(x, y)
            res_part1 += area * perimeter
            res_part2 += area * sides

print(f"res_part1={res_part1}")
print(f"res_part2={res_part2}")

