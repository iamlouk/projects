#!/usr/bin/python

import taichi as ti
import numpy as np

ti.init(arch=ti.cpu)

N = 400

Vec3 = ti.types.vector(n=3, dtype=ti.f32)
A = ti.Vector.field(n=3, dtype=ti.f32, shape=(N, N))
B = ti.Vector.field(n=3, dtype=ti.f32, shape=(N, N))

# initial_state = np.random.rand(N, N, 3).astype(np.float32)

initial_state = np.zeros((N, N, 3), dtype=np.float32)
for i, j in np.ndindex((N, N)):
    initial_state[i, j, 0] = 1

def circle(x, y, r):
    for i, j in np.ndindex((N, N)):
        dx, dy = i - x, j - y
        d = np.sqrt(dx*dx+dy*dy)
        if d < r:
            initial_state[i, j, 0] = 0
            initial_state[i, j, 1] = 1


for i in range(6):
    r = np.random.randint(N // 15)
    x = np.random.randint(N - 2*r)
    y = np.random.randint(N - 2*r)
    circle(x + r, y + r, r)

A.from_numpy(initial_state)
B.from_numpy(np.ones((N, N, 3), dtype=np.float32))

@ti.func
def surrounding_sum(AorB: ti.template(), i: int, j: int, ringsize: ti.template()) -> Vec3:
    sum = ti.Vector([0., 0., 0.], dt=ti.f32)
    for di, dj in ti.ndrange(ringsize, ringsize):
        di = di - ringsize // 2
        dj = dj - ringsize // 2
        x, y = i + di, j + dj
        d: ti.f32 = ti.sqrt(di*di + dj*dj)

        # branch-less anti-aliasing (no idea what the compiler makes of it...):
        is_outside = d - 0.5 > (ringsize / 2.)
        is_inside  = d + 0.5 < (ringsize / 2.)
        on_edge    = (not is_outside) and (not is_inside)
        f: ti.f32 = 1. * float(is_inside and d > 0.) + (1. * float(on_edge) * ((ringsize / 2.) + 0.5 - d))
        # print("di:", di, ", dj:", dj, "f:", f)

        if x < 0:
            x = N - x
        elif x >= N:
            x = x - N
        if y < 0:
            y = N - y
        elif y >= N:
            y = y - N

        if ti.static(AorB):
            sum += A[x, y] * f
        else:
            sum += B[x, y] * f
    return sum

@ti.kernel
def get_normalization(rs: ti.template()) -> ti.f32:
    return surrounding_sum(False, N // 2, N // 2, rs)[0]

RING_SIZE: int = 5
NORMALIZE: ti.f32 = 1. / get_normalization(RING_SIZE)

print("NORMALIZE=", 1./NORMALIZE)

@ti.func
def clamp(x: ti.f32) -> ti.f32:
    return ti.max(0., ti.min(x, 1.))

"""
Works at least somewhat:
next_a = (a + 0.001) * 1.01 - a*b*b
next_b = (b - 0.001) * 0.99 + a*a*b
a = (clamp(next_a) + surrounding[0] * DIFF_SPEED_A) / ti.static(DIFF_SPEED_A + 1.)
b = (clamp(next_b) + surrounding[1] * DIFF_SPEED_B) / ti.static(DIFF_SPEED_B + 1.)
"""

# A: Carrots, B: Bunnies
DIFF_SPEED_A: ti.f32 = 9.0
DIFF_SPEED_B: ti.f32 = 0.1
@ti.func
def transition(cell, surrounding):
    a = cell[0]
    b = cell[1]


    carrots_eaten = a*b*b
    a -= carrots_eaten
    b += carrots_eaten

    a += 0.0025
    b -= 0.0100

    cell[0] = (clamp(a) + surrounding[0] * DIFF_SPEED_A) / ti.static(DIFF_SPEED_A + 1.)
    cell[1] = (clamp(b) + surrounding[1] * DIFF_SPEED_B) / ti.static(DIFF_SPEED_B + 1.)
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
# gui.running = False
AorB = True
while gui.running:
    update(AorB)
    AorB = not AorB
    gui.set_image(A)
    gui.show()

