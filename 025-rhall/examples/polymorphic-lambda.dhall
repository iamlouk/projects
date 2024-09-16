let polymorphicDoTwice = λ(t: Type) -> λ(f: ∀(x: t) -> t, x: t) -> f(f(x))
let intDoTwice: ∀(f: ∀(x: Int) -> Int, x: Int) -> Int = polymorphicDoTwice(Int)
let inc = λ(x: Int) -> x + 1
in
	intDoTwice(inc, 40)
