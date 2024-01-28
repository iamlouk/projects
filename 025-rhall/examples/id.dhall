let id: ∀(t: Type) -> ∀(x: t) -> t = λ(t: Type) -> λ(x: t) -> x in (id(Int) : ∀(x: Int) -> Int)(42)
