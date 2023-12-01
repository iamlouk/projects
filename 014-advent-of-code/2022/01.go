package main

import (
	"bufio"
	"fmt"
	"os"
	"sort"
	"strconv"
	"strings"
)

func main() {
	f, err := os.Open("./01-input.txt")
	if err != nil {
		panic(err)
	}
	defer f.Close()
	input := bufio.NewScanner(f)

	maxcals := -1
	calscarried := make([]int, 0)
	for input.Scan() {
		cals := 0
		for {
			line := strings.Trim(input.Text(), " \t\n")
			if len(line) == 0 {
				break
			}

			val, err := strconv.Atoi(line)
			if err != nil {
				panic(err)
			}
			cals += val

			if !input.Scan() {
				break
			}
		}

		calscarried = append(calscarried, cals)
		if cals > maxcals {
			maxcals = cals
		}
	}

	if err := input.Err(); err != nil {
		panic(err)
	}

	fmt.Printf("max. cals.: %d\n", maxcals)

	// Yes, something like top-k would be a lot better, but I am lazy!
	sort.Sort(sort.IntSlice(calscarried))
	fmt.Printf("top 3:")
	n := len(calscarried)
	for i := n - 3; i < n; i++ {
		fmt.Printf(" %d, ", calscarried[i])
	}
	fmt.Printf("sum: %d\n", calscarried[n-3]+calscarried[n-2]+calscarried[n-1])
}
