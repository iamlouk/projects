#!/usr/bin/env python3
import sys, re

input = [list(line.strip()) for line in sys.stdin]

N, M = len(input), len(input[0])

def get(i: int, j: int) -> str:
    if 0 <= i < N and 0 <= j < M:
        return input[i][j]

def count(word: str) -> int:
    total = 0
    wlen = len(word)
    for i in range(0, N):
        for j in range(0, M):
            for k in range(0, wlen):
                if get(i, j + k) != word[k]:
                    break
            else:
                print(f"row: i={i},j={j} -> {word}")
                total += 1

    for i in range(0, N):
        for j in range(0, M):
            for k in range(0, wlen):
                if get(i + k, j) != word[k]:
                    break
            else:
                print(f"col: i={i},j={j} -> {word}")
                total += 1

    for i in range(0, N):
        for j in range(0, M):
            for k in range(0, wlen):
                if get(i + k, j + k) != word[k]:
                    break
            else:
                print(f"diag1: i={i},j={j} -> {word}")
                total += 1

    for i in range(0, N):
        for j in range(0, M):
            for k in range(0, wlen):
                if get(i + k, j - k) != word[k]:
                    break
            else:
                print(f"diag2: i={i},j={j} -> {word}")
                total += 1

    return total



print(input)
total = 0
total += count("XMAS")
total += count("SAMX")
print(total)
