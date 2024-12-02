#!/usr/bin/env python3

import sys
from functools import reduce

lines = [list(map(int, line.split(' '))) for line in sys.stdin]
issafe = lambda nums: reduce(lambda red, num: (red[0] and (1 <= (num - red[1]) <= 3), num), nums, (True, nums[0] - 1))[0]
print(sum([(1 if issafe(list(row)) or issafe(list(reversed(row))) else 0) for row in lines]))
