#!/usr/bin/env python3
import sys, re

print(sum([
    int(cg[0]) * int(cg[1]) for cg in
        re.compile(r'mul\(([0-9]{1,3}),([0-9]{1,3})\)').findall(sys.stdin.read())
]))

