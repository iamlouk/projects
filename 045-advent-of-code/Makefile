.PHONY: day01 day02 day03 day04 day05 day06 day07 day08 day09 day10 day11 day12 day13 day14

day01: ./01-input.txt
	# A 100% https://jqlang.github.io/jq solution. The range chunk hack was found on stack overflow.
	jq -s '[range(length/2|ceil) as $$i | .[2*$$i:2*$$i+2]] | { a: ([.[][0]] | sort), b: ([.[][1]] | sort) } | transpose | .[] | (.[0] - .[1])' $< \
		| jq -s '[.[] | fabs] | add'

day02: ./02.py ./02-input.txt
	python ./02.py < ./02-input.txt

day03: ./03.py ./03-input.txt
	python ./03.py < ./03-input.txt

day04: ./04.py ./04-input.txt
	python ./04.py < ./04-input.txt

day05: ./05.go ./05-input.txt
	go run ./05.go < ./05-input.txt

day06: ./06.py ./06-input.txt
	python ./06.py < ./06-input.txt

day07: ./07.py ./07-input.txt
	python ./07.py < ./07-input.txt

day08: ./08.py ./08-input.txt
	python ./08.py < ./08-input.txt

day09: ./09.py ./09-input.txt
	python ./09.py < ./09-input.txt

day10: ./10.py ./10-input.txt
	python ./10.py < ./10-input.txt

day11: ./11.py ./11-input.txt
	python ./11.py < ./11-input.txt

day12: ./12.py ./12-input.txt
	python ./12.py < ./12-input.txt

day13: ./13.py ./13-input.txt
	python ./13.py < ./13-input.txt

day14: ./14.py ./14-input.txt
	python ./14.py < ./14-input.txt
