from badjit import jit

@jit
def add(x: int, y: int) -> int:
    res = x + y
    return res

print(add(1, 2))

