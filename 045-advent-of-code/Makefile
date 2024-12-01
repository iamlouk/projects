.PHONY: day01

day01: ./01-input.txt
	# A 100% https://jqlang.github.io/jq solution. The range chunk hack was found on stack overflow.
	jq -s '[range(length/2|ceil) as $$i | .[2*$$i:2*$$i+2]] | { a: ([.[][0]] | sort), b: ([.[][1]] | sort) } | transpose | .[] | (.[0] - .[1])' $< \
		| jq -s '[.[] | fabs] | add'
