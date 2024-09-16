#!/usr/bin/python

import taichi as ti
import numpy as np

ti.init(arch=ti.cpu)

N = 400

Vec3 = ti.types.vector(n=3, dtype=ti.f32)
A = ti.Vector.field(n=3, dtype=ti.f32, shape=(N, N))
B = ti.Vector.field(n=3, dtype=ti.f32, shape=(N, N))

initial_state = np.random.rand(N, N, 3).astype(np.float32)

A.from_numpy(initial_state)
B.from_numpy(np.ones((N, N, 3), dtype=np.float32))

@ti.func
def surrounding_sum(AorB: ti.template(), i: int, j: int, ringsize: ti.template()) -> Vec3:
    sum = ti.Vector([0., 0., 0.], dt=ti.f32)
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

        if ti.static(AorB):
            sum += A[x, y] * f
        else:
            sum += B[x, y] * f
    return sum

@ti.kernel
def get_normalization(rs: ti.template()) -> ti.f32:
    return surrounding_sum(False, N // 2, N // 2, rs)[0]

RING_SIZE: int = 2
NORMALIZE: ti.f32 = 1. / get_normalization(RING_SIZE)

@ti.func
def clamp(x: ti.f32) -> ti.f32:
    return ti.max(0., ti.min(x, 1.))

# A: Carrots, B: Bunnies
GROWTH_RATE_A: ti.f32 =  0.1
GROWTH_RATE_B: ti.f32 =  0.15
DIFF_SPEED_A: ti.f32  = 1.5
DIFF_SPEED_B: ti.f32  = 0.5
@ti.func
def transition(cell, surrounding):
    a = (cell[0] + surrounding[0] * DIFF_SPEED_A) / ti.static(DIFF_SPEED_A + 1.)
    b = (cell[1] + surrounding[1] * DIFF_SPEED_B) / ti.static(DIFF_SPEED_B + 1.)

    carrots_eaten = a * b * b
    carrots_grown = GROWTH_RATE_A * (1. - a)

    bunnies_died = GROWTH_RATE_B * b
    bunnies_grown = carrots_eaten

    cell[0] = clamp(a - carrots_eaten + carrots_grown)
    cell[1] = clamp(b - bunnies_died + bunnies_grown)
    cell[2] = 0.
    return cell

@ti.kernel
def update(AorB: ti.template()):
    for i, j in ti.ndrange(N, N):
        sum = surrounding_sum(AorB, i, j, RING_SIZE) * NORMALIZE
        if any(sum < 0.) or any(1. < sum):
            print("Fuck!")

        cell = ti.Vector([0., 0., 0.], dt=ti.f32)
        if ti.static(AorB):
            cell = A[i, j]
        else:
            cell = B[i, j]
        if any(cell < 0.) or any(1. < cell):
            print("Fuck!")

        new_state = transition(cell, sum)
        if ti.static(AorB):
            B[i, j] = new_state
        else:
            A[i, j] = new_state


gui = ti.GUI("Taichi CA Test", res=(N, N))
AorB = True
while gui.running:
    update(AorB)
    AorB = not AorB
    gui.set_image(A)
    gui.show()

