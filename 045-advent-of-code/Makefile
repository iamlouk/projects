.PHONY: day01 day02 day03 day04 day05 day06 day07 day08

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
