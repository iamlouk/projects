import ctypes
from badjit import jit

@jit
def add(a: int, b: int) -> int:
    sum = a + b
    return sum

assert add(1, 2) == 3

@jit
def fib(n: int) -> int:
    a = 1
    b = 1
    while n > 0:
        tmp = a + b
        a = b
        b = tmp
        n = n - 1
    return b

for (n, expected) in zip(range(0, 10), [1, 2, 3, 5, 8, 13, 21, 34, 55, 89]):
    assert fib(n) == expected

@jit
def fib_rec(n: int) -> int:
    if n < 1:
        return 1
    else:
        return fib_rec(n - 1) + fib_rec(n - 2)

for (n, expected) in zip(range(0, 10), [1, 2, 3, 5, 8, 13, 21, 34, 55, 89]):
    assert fib_rec(n) == expected


# TODO:...
if False:
    @jit
    def sum(n: int, data: list[float]) -> float:
        s = 0.0
        for i in range(0, n):
            s = s + data[i]
        return s




