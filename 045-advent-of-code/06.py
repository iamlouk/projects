#!/usr/bin/env python3
import sys, enum

map = [list(line.strip()) for line in sys.stdin]

class Dir(enum.Enum):
    UP = 'UP'
    RIGHT = 'RIGHT'
    DOWN = 'DOWN'
    LEFT = 'LEFT'

    def rotate(self):
        if self == Dir.UP:
            return Dir.RIGHT
        if self == Dir.RIGHT:
            return Dir.DOWN
        if self == Dir.DOWN:
            return Dir.LEFT
        if self == Dir.LEFT:
            return Dir.UP

    def npos(self, pos):
        if self == Dir.UP:
            return (pos[0] - 1, pos[1])
        if self == Dir.RIGHT:
            return (pos[0], pos[1] + 1)
        if self == Dir.DOWN:
            return (pos[0] + 1, pos[1])
        if self == Dir.LEFT:
            return (pos[0], pos[1] - 1)

visited: set[tuple[int, int]] = set()

pos = (-1, -1)
dir = Dir.UP

rows = len(map)
cols = len(map[0])
for i in range(0, rows):
    assert len(map[i]) == rows
    for j in range(0, cols):
        if map[i][j] == '^':
            pos = (i, j)

assert pos[0] != -1 and pos[1] != -1

pathlen = 0
while True:
    print(f"pos=({pos[0]},{pos[1]}), dir={dir}, pathlen={pathlen}")
    if not(0 <= pos[0] < rows) or not(0 <= pos[1] < cols):
        break
    visited.add(pos)
    assert map[pos[0]][pos[1]] != '#'

    pathlen += 1
    while True:
        npos = dir.npos(pos)
        if 0 <= npos[0] < rows and 0 <= npos[1] < cols and map[npos[0]][npos[1]] == '#':
            dir = dir.rotate()
            continue
        pos = npos
        break

print(f"pathlen={pathlen}, pathsize={len(visited)}")

