let dontimes = λ(f: ∀(x: Int) -> Int, x: Int, n: Int) ->
	if n == 0 then x else f(dontimes(f, x, n - 1))
in dontimes(λ(x: Int) -> x + x, 1, 10)
