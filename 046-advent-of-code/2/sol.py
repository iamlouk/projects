#!/usr/bin/env python3

def isinvalid(x: int) -> bool:
    x = str(x)
    if len(x) % 2 == 1:
        return False

    mid = len(x)//2
    a, b = x[0:mid], x[mid:]
    return a == b

invalid = 0

with open("/dev/stdin") as f:
    for line in f:
        line = line.strip()
        if line == "":
            continue

        for pair in line.split(','):
            pair = pair.split('-')
            a, b = int(pair[0]), int(pair[1])
            print(f"a: {a}, b: {b}")

            for x in range(a, b+1):
                if (isinvalid(x)):
                    print(f"invalid: {x}")
                    invalid += x

print(invalid)
