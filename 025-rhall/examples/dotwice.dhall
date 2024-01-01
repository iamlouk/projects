let dotwice = λ(f: ∀(x: Int) -> Int, x: Int) -> f(f(x)) in dotwice(λ(x: Int) -> x * 2, 2)
