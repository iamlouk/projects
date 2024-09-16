#!/usr/bin/python

import taichi as ti
import numpy as np

ti.init(arch=ti.cpu)

N = 400

A = ti.field(dtype=ti.f32, shape=(N, N))
B = ti.field(dtype=ti.f32, shape=(N, N))

initial_state = np.random.rand(N, N).astype(np.float32)
A.from_numpy(initial_state)
B.from_numpy(np.ones((N, N), dtype=np.float32))


RADIUS_INNER: int =  7
RADIUS_OUTER: int = 16
NORMALIZE_INNER: ti.f32 = 0
NORMALIZE_OUTER: ti.f32 = 0
USE_A: bool = True
USE_B: bool = False

@ti.func
def surrounding_sum(AorB: ti.template(), i: int, j: int, ringsize: ti.template()) -> ti.f32:
    sum: ti.f32 = 0
    for di, dj in ti.ndrange(ringsize * 2, ringsize * 2):
        di = di - ringsize
        dj = dj - ringsize
        x, y = i + di, j + dj
        d: ti.f32 = ti.sqrt(di*di + dj*dj)

        # branch-less anti-aliasing (no idea what the compiler makes of it...):
        is_outside = d - 0.5 > ringsize
        is_inside  = d + 0.5 < ringsize
        on_edge    = (not is_outside) and (not is_inside)
        f: ti.f32 = 1. * float(is_inside) + (1. * float(on_edge) * (ringsize + 0.5 - d))
        # print("ringsize:", ringsize, ", di:", di, ", dj:", dj, ", d:", d, ", f:", f, "inside:", is_inside, ", outside:", is_outside)

        if x < 0:
            x = N - x
        if x >= N:
            x = x - N
        if y < 0:
            y = N - y
        if y >= N:
            y = y - N

        if ti.static(AorB == USE_A):
            sum += A[x, y] * f
        else:
            sum += B[x, y] * f
    return sum

@ti.kernel
def calc_normalize_factor(ringsize: int) -> ti.f32:
    return surrounding_sum(USE_B, N // 2, N // 2, ringsize)

NORMALIZE_INNER = 1. / calc_normalize_factor(RADIUS_INNER)
NORMALIZE_OUTER = 1. / (calc_normalize_factor(RADIUS_OUTER) - (1. / NORMALIZE_INNER))

print("1./NORMALIZE_INNER=", 1./NORMALIZE_INNER)
print("1./NORMALIZE_OUTER=", 1./NORMALIZE_OUTER)

DEATH_LOWER = 0.267
DEATH_UPPER = 0.445

@ti.func
def sigmoid(x: ti.f32) -> ti.f32:
    return 1. / (1. + ti.exp(-32.*x))

@ti.func
def cell_transition(x1: ti.f32, x2: ti.f32) -> ti.f32:
    # TODO...
    return sigmoid(x1 - x2) + sigmoid(x2 - x1)

@ti.kernel
def update(AorB: ti.template()):
    for i, j in A:
        inner_sum = surrounding_sum(AorB, i, j, RADIUS_INNER)
        outer_sum = surrounding_sum(AorB, i, j, RADIUS_OUTER) - inner_sum
        inner_sum *= NORMALIZE_INNER
        outer_sum *= NORMALIZE_OUTER

        new_state: ti.f32 = cell_transition(inner_sum, outer_sum)
        if ti.static(AorB == USE_A):
            A[i, j] = new_state
        else:
            B[i, j] = new_state

gui = ti.GUI("Taichi CA Test", res=(N, N))
AorB = USE_A
while gui.running:
    update(AorB)
    AorB = not AorB
    gui.set_image(A)
    gui.show()

