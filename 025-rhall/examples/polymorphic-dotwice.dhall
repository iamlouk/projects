let polymorphicDoTwice: ∀(t: Type) -> ∀(f: ∀(x: t) -> t, x: t) -> t = λ(t: Type) -> λ(f: ∀(x: t) -> t, x: t) -> f(f(x))
let intDoTwice = polymorphicDoTwice(Int)
let intPow2 = λ(x: Int) -> x * x
in intDoTwice(intPow2, 2)
