#!/usr/bin/env python3

import math

pos: int = 50
atzero: int = 0
throughzero: int = 0

with open("/dev/stdin") as f:
    for line in f:
        line = line.strip()
        if line == "":
            continue

        prevpos = pos
        dir = line[0]
        rot = int(line[1:])
        assert dir == "L" or dir == "R"
        # assert 0 <= rot <= 99
        if dir == "L":
            rot = -rot

        pos += rot
        div = math.ceil(abs(rot) / 100)
        pos = pos % 100
        print(f"pos: {prevpos} + {rot} -> {pos}, div: {div}")

        throughzero += div
        
        if pos == 0:
            atzero += 1

print(f"at zero: {atzero}, through zero: {throughzero}")

