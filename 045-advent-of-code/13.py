#!/usr/bin/env python
import sys, math

def parse(line: str) -> (int, int):
    x, y = line.split(': ')[1].split(', ')
    x, y = int(x[2:]), int(y[2:])
    return x, y

res = 0
epsilon = 0.000001
for (p1, p2, p3) in map(lambda x: x.strip().split('\n'), sys.stdin.read().split('\n\n')):
    assert p1.startswith('Button A:')
    assert p2.startswith('Button B:')
    assert p3.startswith('Prize:')
    ax, ay = parse(p1)
    bx, by = parse(p2)
    x, y = parse(p3)
    print(f"a=({ax}, {ay}), b=({bx}, {by}), prize=({x}, {y})")
    # math.... -.-
    numa = ((y * bx) - (x * by)) / ((ay * bx) - (by * ax))
    numb = (x - (ax * numa)) / bx
    if abs(numa - round(numa)) < epsilon and abs(numb - round(numb)) < epsilon:
        print(f"-> #A = {numa}, #B = {numb}")
        res += 3 * round(numa)
        res += 1 * round(numb)

print(f"res: {res}")

