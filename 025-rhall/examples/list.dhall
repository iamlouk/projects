-- I know that this is ugly and hacky and bad, but it is what it is,
-- because a lot of language features to make this stuff work out of
-- the box a lot easier/nicer are not there yet.

let List = λ(T: Type) -> Any

let Nil  = λ(T: Type) -> ⊥ as Any

let Cons = λ(T: Type) -> λ(x: T, xs: List(T)) ->
           ({ value = x, next = xs }) as Any

let List/fold: ∀(A: Type, B: Type) -> ∀(list: List(A), f: ∀(acc: B, x: A) -> B, x0: B) -> B
             = λ(A: Type, B: Type) -> λ(list: List(A), f: ∀(acc: B, x: A) -> B, x0: B) ->
               if Option/isSome({ value: A, next: Any })(list as { value: A, next: Any })
               then (let head = Option/unwrap({ value: A, next: Any })
                                             (list as { value: A, next: Any })
                     in List/fold(A, B)(head.next, f, f(x0, head.value)))
               else x0

let numbers = (
    let NilI: List(Int) = Nil(Int)
    let ConsI: ∀(x: Int, xs: List(Int)) -> List(Int) = Cons(Int)
    in ConsI(1, ConsI(2, ConsI(3, ConsI(4, ConsI(5, NilI)))))) : List(Int)

in {
    numbers = List/fold(Int, Text)(numbers, λ(s: Text, x: Int) -> s + ", " + (x as Text), "0"),
    sum = List/fold(Int, Int)(numbers, λ(x: Int, y: Int) -> x + y, 0)
}
