#!/usr/bin/env python3
import sys, enum, itertools

total = 0
for line in sys.stdin:
    parts = line.split(': ')
    expected = int(parts[0])
    operands = list(map(lambda x: int(x.strip()), parts[1].split(' ')))
    print(f"expected={expected}, operands={operands}")

    for permutation in itertools.product(['*', '+'], repeat=(len(operands) - 1)):
        res = operands[0]
        for i, op in enumerate(permutation):
            if op == '+':
                res = res + operands[i+1];
            elif op == '*':
                res = res * operands[i+1];
            else:
                raise "WTF?"
        if res == expected:
            break
    else:
        continue
    total += expected

print(total)

