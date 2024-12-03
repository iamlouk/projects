#!/usr/bin/env python3
import sys, re
from functools import reduce

e = re.compile(r'mul\(([0-9]{1,3}),([0-9]{1,3})\)')
print(reduce(lambda res, match: res + (int(match[0]) * int(match[1])), e.findall(sys.stdin.read()), 0))

