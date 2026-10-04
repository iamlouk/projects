#!/usr/bin/env python3

import math

lines = []
with open("/dev/stdin") as f:
    for line in f:
        line = line.strip()
        if line == "":
            continue

        lines.append(tuple([int(c) for c in line.split(',')]))

def area(a, b):
    w = abs(a[0]-b[0]) + 1
    h = abs(a[1]-b[1]) + 1
    return w * h

maxarea = 0
n = len(lines)
for i in range(0, n):
    for j in range(i):
        a, b = lines[i], lines[j]
        size = area(a, b)
        print(f"{a} x {b} -> {size}")
        maxarea = maxarea if maxarea > size else size


print(maxarea)
