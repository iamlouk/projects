#!/usr/bin/python

import taichi as ti
import numpy as np

ti.init(arch=ti.cpu)

N = 400

Vec3 = ti.types.vector(n=3, dtype=ti.f32)
A = ti.Vector.field(n=3, dtype=ti.f32, shape=(N, N))
B = ti.Vector.field(n=3, dtype=ti.f32, shape=(N, N))

initial_state = np.zeros((N, N, 3), dtype=np.float32)
for i, j in np.ndindex((N, N)):
    initial_state[i, j, 0] = 1

def circle(cx, cy, r):
    for i, j in np.ndindex((r*2, r*2)):
        i, j = i - r, j - r
        d = np.sqrt(i*i+j*j)
        if d > r:
            continue

        x, y = cx + i, cy + j
        if x < 0:
            x = N + x
        elif x >= N:
            x = x - N
        if y < 0:
            y = N + y
        elif y >= N:
            y = y - N

        initial_state[x, y, 0] = 0
        initial_state[x, y, 1] = 1

for i in range(25):
    r = np.random.randint(N // 5)
    x = np.random.randint(N - 2*r)
    y = np.random.randint(N - 2*r)
    circle(x + r, y + r, r)

A.from_numpy(initial_state)
B.from_numpy(np.ones((N, N, 3), dtype=np.float32))

@ti.func
def clamp(x: ti.template()):
    return ti.max(0., ti.min(x, 1.))

@ti.func
def surrounding_sum(AorB: ti.template(), i: int, j: int) -> Vec3:
    sum = ti.Vector([0., 0., 0.], dt=ti.f32)
    for di, dj in ti.ndrange(3, 3):
        x, y = i + di - 1, j + dj - 1
        if x < 0:
            x = N + x
        elif x >= N:
            x = x - N
        if y < 0:
            y = N + y
        elif y >= N:
            y = y - N

        f: ti.f32 = 1. * float(not(x == i and y == j))
        if ti.static(AorB): sum += A[x, y] * f
        else:               sum += B[x, y] * f

    return sum

DIFFUSION_SPEEDS = ti.Vector([1., .2, 0.], dt=ti.f32)

@ti.kernel
def update(AorB: ti.template()):
    for i, j in ti.ndrange(N, N):
        cell = A[i, j] if ti.static(AorB) else B[i, j]
        surrounding = surrounding_sum(AorB, i, j)
        cell += (-1.*cell + (1./8.) * surrounding) * DIFFUSION_SPEEDS
        carrots, bunnies = cell[0], cell[1]

        carrots_eaten = carrots * bunnies * bunnies
        carrots -= carrots_eaten
        carrots += 0.005
        bunnies = (bunnies - 0.01) * 0.9975
        bunnies += carrots_eaten

        cell[0], cell[1] = carrots, bunnies
        if ti.static(AorB): B[i, j] = clamp(cell)
        else:               A[i, j] = clamp(cell)

gui = ti.GUI("Taichi CA Test", res=(N, N))
AorB = True
i = 0
while gui.running:
    update(AorB)
    AorB = not AorB
    gui.set_image(A)
    gui.show()

    if i % 100 == 0:
        norm = A.to_numpy(dtype=np.float32).sum(axis=(0, 1)) / (N * N)
        print("normalized sum:", norm)
    i += 1
    # gui.running = False


