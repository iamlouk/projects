.PHONY: day01 day02

day01: ./01/trebuchet.f90 ./01/input.txt
	gfortran -Wall -O1 ./01/trebuchet.f90 -o ./01/solver
	./01/solver < ./01/input.txt

day02: ./02/cube-conundrum.hs ./02/input.txt
	runhaskell ./02/cube-conundrum.hs < ./02/input.txt

