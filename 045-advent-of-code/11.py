#!/usr/bin/env python
import sys

input = [int(part) for part in sys.stdin.read().strip().split(' ')]
print(f"input={input}")

def numdigits(n: int) -> int:
    # This is shitty but works, calculating base 10 log would
    # probably be slower anyways!
    assert n > 0
    if n < 10: return 1
    if n < 100: return 2
    if n < 1000: return 3
    if n < 10000: return 4
    if n < 100000: return 5
    if n < 1000000: return 6
    if n < 10000000: return 7
    if n < 100000000: return 8
    if n < 1000000000: return 9
    if n < 10000000000: return 10
    if n < 100000000000: return 11
    if n < 1000000000000: return 12
    if n < 10000000000000: return 13
    if n < 100000000000000: return 14
    if n < 1000000000000000: return 15
    assert False

def blink(nums: list[int]) -> list[int]:
    nnums = []
    for n in nums:
        if n == 0:
            nnums.append(1)
            continue

        ndigits = numdigits(n)
        if ndigits % 2 == 0:
            d = 10**(ndigits // 2)
            a, b = n // d, int(n % d)
            nnums.append(a)
            nnums.append(b)
            continue

        nnums.append(n * 2024)

    return nnums

nums = input
for i in range(25):
    nums = blink(nums)
    # print(f"blink #{i+1}: {nums}")

print(f"#stones = {len(nums)}")
