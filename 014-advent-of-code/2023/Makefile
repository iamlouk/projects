.PHONY: day01 day02 day03 day04 day05 day06 day07 day08 day09


# Languages already used:
# - Fortran, Haskell, D, Bash, JavaScript, C, Rust,
#   Scala(3), Clojure, Ruby
# Languages still to use:
# - Go, C++, Swift, Python

# Fortran may not be as bad as I thought and actually
# pretty useful sometimes, but text processing is not
# it's strong point!
day01: ./01/trebuchet.f90 ./01/input.txt
	gfortran -Wall -O1 ./01/trebuchet.f90 -o ./01/solver
	./01/solver < ./01/input.txt

# Haskell... Actually super nice, but without a lot of
# the syntactic sugar constructs, it is pretty verbose.
day02: ./02/cube-conundrum.hs ./02/input.txt
	runhaskell ./02/cube-conundrum.hs < ./02/input.txt

# I actually like D, would like to use it more, its just
# the "foreach (...;...)" syntax I hate!
day03: ./03/gear-ratios.d ./03/input.txt
	gdc -Wall -Wextra -O1 ./03/gear-ratios.d -o ./03/solver
	./03/solver < ./03/input.txt

# Doing this in bash was actually not as hard as I thought...
day04: ./04/scratchcards.sh ./04/input.txt
	bash ./04/scratchcards.sh < ./04/input.txt

# Does the job...
day05:
	node ./05/fertilizer.mjs < ./05/input.txt

# C is great (sometimes)!
day06:
	gcc -Wall -Wextra -fsanitize=address -O1 -g -fopenmp ./06/waitforit.c -o ./06/solver
	./06/solver < ./06/input.txt

# Rust is great (mostly)!
day07:
	rustc -C opt-level=1 -C debuginfo=1 ./07/camelcards.rs -o ./07/solver
	./07/solver < ./07/input.txt

# Scala is alright, but WTF, those compile times are horrible!
# Also, this was my first time programming in scala3, it is
# slightly better than scala2, but not fucking worth a python2/python3 shitshow.
day08:
	scala3 -explain ./08/wasteland.scala < ./08/input.txt

# Clojure is even better than I remember it, I should use it more!
day09:
	clojure -M ./09/oasis.clj < ./09/input.txt

# I am not a fan of ruby...
day10:
	ruby ./10/pipemaze.rb < ./10/input.txt

