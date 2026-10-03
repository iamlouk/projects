#!/usr/bin/env python3

import math

lines = []
with open("/dev/stdin") as f:
    for line in f:
        line = line.strip()
        if line == "":
            continue

        lines.append(tuple([int(c) for c in line.split(',')]))

def dist(a, b):
    return math.sqrt((a[0]-b[0])**2 + (a[1]-b[1])**2 + (a[2]-b[2])**2)

distances = []
for i in range(len(lines)):
    for j in range(i):
        a, b = lines[i], lines[j]
        d = dist(a, b)
        distances.append((d, a, b, i, j))

distances.sort(key=lambda x: -x[0])


islandsbyline = [frozenset([i]) for i in range(len(lines))]
islands = set(islandsbyline)

def union(i, j):
    set_i = islandsbyline[i]
    set_j = islandsbyline[j]

    if set_i is set_j or set_i == set_j:
        return

    islands.remove(set_i)
    islands.remove(set_j)

    merged = set_i | set_j
    for element in merged:
        islandsbyline[element] = merged

    islands.add(merged)

def connect():
    d, a, b, i, j = distances.pop()
    for island in islands:
        if i in island:
            union(i, j)
            break
        if j in island:
            union(i, j)
            break

    return (a, b, d)

for n in range(1000):
    a, b, d = connect()
    print(f"#{n}: {b} <-> {a} (dist.: {d})")

islands = list(islands)
islands.sort(key=lambda x: -len(x))
print(islands)
print(len(islands[0]) * len(islands[1]) * len(islands[2]))
