#!/usr/bin/env python
import sys, math

N, M = 101, 103
# N, M = 11, 7
SECONDS = 100
q1, q2, q3, q4 = 0, 0, 0, 0
for line in sys.stdin:
    assert line.startswith('p=')
    a, b = line.strip().split(' ')
    px, py = map(int, a[2:].split(','))
    vx, vy = map(int, b[2:].split(','))
    x = (px + vx * SECONDS) % N
    y = (py + vy * SECONDS) % M
    if x < (N // 2) and y < (M // 2):
        print(f"p0=({px}, {py}), v=({vx}, {vy}), p1=({x}, {y}) -> Q1")
        q1 += 1
    elif x > (N // 2) and y < (M // 2):
        print(f"p0=({px}, {py}), v=({vx}, {vy}), p1=({x}, {y}) -> Q2")
        q2 += 1
    elif x < (N // 2) and y > (M // 2):
        print(f"p0=({px}, {py}), v=({vx}, {vy}), p1=({x}, {y}) -> Q3")
        q3 += 1
    elif x > (N // 2) and y > (M // 2):
        print(f"p0=({px}, {py}), v=({vx}, {vy}), p1=({x}, {y}) -> Q4")
        q4 += 1
    else:
        print(f"p0=({px}, {py}), v=({vx}, {vy}), p1=({x}, {y}) -> On diag.")

print(f"q1={q1}, q2={q2}, q3={q3}, q4={q4},res={q1*q2*q3*q4}")
