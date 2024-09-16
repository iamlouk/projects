-- I would to get typecheck and eval and so on so far that this successfully typechecks
-- and that the printed value/result of this is: λ(f: ∀(x: Int) -> Int, x: Int) -> f(f(x)).
-- Currently, there is no typecheck and instead of Int, t is printed (even after currying/partial eval).
-- I guess what the orig haskell dhall does is, after the first apply, just replace all `t`s by letting the
-- apply return a copied AST with t replaced. I do not want to do this because I have recursion and I prefer
-- having the types as a scoped variable instead of duplicating. Because of the RC's, duplication would
-- not be that expensive I guess, but still. This makes it tricky though... Have the eval lookup types in the env?
-- Have display check if the type t is also known by another name (e.g. Int here)?

let polymorphicDoTwice = λ(t: Type) -> λ(f: ∀(x: t) -> t, x: t) -> f(f(x))
in
  polymorphicDoTwice(Int)

