let opt = Some(Int)(42)
let is42 = λ(x: Int) -> x == 42
in
	Option/fold (Int, Bool) (opt) (is42, false)
