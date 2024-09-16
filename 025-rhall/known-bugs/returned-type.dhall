-- works: let GetTyp = λ(_: Int) -> Int in 42 : GetTyp(123)
-- broken:
let GetTyp = λ(A: Type) -> A in 42 : GetTyp(Int)
