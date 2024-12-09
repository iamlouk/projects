#!/usr/bin/env python3
import sys, enum, itertools, math

input = [int(c) for c in sys.stdin.read().strip()]
expanded = []
for i, n in enumerate(input):
    id = i // 2 if i % 2 == 0 else -1
    for _ in range(0, n):
        expanded.append(id)

print(input)



firstgap, endoflastfile = expanded.index(-1), len(expanded) - 1
while firstgap < endoflastfile:
    print(f"start={firstgap}, end={endoflastfile}")
    assert firstgap < len(expanded) and expanded[firstgap] == -1
    assert endoflastfile < len(expanded) and expanded[endoflastfile] != -1

    expanded[firstgap] = expanded[endoflastfile]
    expanded[endoflastfile] = -1
    endoflastfile -= 1
    while expanded[endoflastfile] == -1:
        endoflastfile -= 1
    firstgap += 1
    while expanded[firstgap] != -1:
        firstgap += 1

checksum = 0
for i, n in enumerate(expanded):
    if n == -1:
        break
    checksum += i * n
print(f"checksum={checksum}")
