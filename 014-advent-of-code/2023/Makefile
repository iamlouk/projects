.PHONY: day01 day02 day03 day04 day05 day06

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

day05:
	node ./05/fertilizer.mjs < ./05/input.txt

day06:
	gcc -Wall -Wextra -fsanitize=address -O1 -g -fopenmp ./06/waitforit.c -o ./06/waitforit
	./06/waitforit < ./06/input.txt

