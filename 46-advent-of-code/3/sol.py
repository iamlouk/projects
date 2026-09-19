#!/usr/bin/env python3

sum = 0

with open("/dev/stdin") as f:
    for line in f:
        line = line.strip()
        if line == "":
            continue

        line = [int(x) for x in line]

        max1 = 0
        maxidx = 0
        for i in range(len(line)-1):
            x = line[i]
            if x > max1:
                max1 = x
                maxidx = i

        max2 = max(line[(maxidx+1):])
        print(f"max: {max1} {max2}")

        sum += max1 * 10 + max2

print(sum)
