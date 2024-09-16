Hey, the type-checker is fixed! __But there are memory-leaks!__ The problem is that a lambda holds a reference to it's scope, and the scope contains the lambda. This is ugly and hacky and leaks memory, but it works. The scope mechanism (and lambda value) needs improvement anyways...! I want to implement a garbage collector one day, and I am sure there are some cool things that can be done using the fact that a scope is immutable.

### TODOs

- Perf. improvements around scopes and lambdas (closures to be precise)
- Fix memory leaks (GC for closures?)
- Generic lists? Other generic stuff?
- tagged-union/enum types
- ...
