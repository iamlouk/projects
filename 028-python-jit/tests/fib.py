from badjit import jit

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

for n in range(0, 10):
    print(fib(n))

