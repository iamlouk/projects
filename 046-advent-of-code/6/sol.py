#!/usr/bin/env python3

lines = []
with open("/dev/stdin") as f:
    for line in f:
        line = line.strip()
        if line == "":
            continue

        lines.append(line.split())

add = lambda x, y: x + y
mul = lambda x, y: x * y

ops = [(add if op == '+' else mul) for op in lines.pop()]
nums = [[int(num) for num in line] for line in lines]

print(ops)
print(nums)

sum = 0
for i, op in enumerate(ops):
    x = nums[0][i]
    for j in range(1, len(nums)):
        x = op(x, nums[j][i])
    print(x)
    sum += x

print(sum)

