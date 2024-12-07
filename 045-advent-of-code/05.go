package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

var orderingsBefore2After = map[int]map[int]bool{}
var orderingsAfter2Before = map[int]map[int]bool{}

func iscorrect(nums []int) bool {
	for pos, num := range nums {
		// fmt.Printf("checking: pos=%v, num=%v\n", pos, num)
		b2a := orderingsBefore2After[num]
		a2b := orderingsAfter2Before[num]
		for i := 0; i < pos; i++ {
			if _, ok := a2b[nums[i]]; !ok {
				// fmt.Printf("pos_lower=%v, a2b[nums[pos_lower]]=%v\n", i, a2b[nums[i]])
				return false
			}
		}

		for i := pos + 1; i < len(nums); i++ {
			if _, ok := b2a[nums[i]]; !ok {
				// fmt.Printf("pos_higher=%v, b2a[nums[pos_higher]]=%v\n", i, b2a[nums[i]])
				return false
			}
		}
	}
	return true
}

func main() {
	r := bufio.NewScanner(os.Stdin)

	for i := 0; i < 100; i++ {
		orderingsBefore2After[i] = make(map[int]bool)
		orderingsAfter2Before[i] = make(map[int]bool)
	}

	for r.Scan() {
		line := r.Text()
		if len(line) == 0 {
			break
		}

		parts := strings.Split(line, "|")
		b, _ := strconv.Atoi(parts[0])
		a, _ := strconv.Atoi(parts[1])

		orderingsBefore2After[b][a] = true
		orderingsAfter2Before[a][b] = true
	}

	// fmt.Printf("b2a: %#v\n\n", orderingsBefore2After)
	// fmt.Printf("a2b: %#v\n\n", orderingsAfter2Before)

	sum := 0
	for r.Scan() {
		parts := strings.Split(r.Text(), ",")
		nums := []int{}
		for _, part := range parts {
			n, _ := strconv.Atoi(part)
			nums = append(nums, n)
		}
		correct := iscorrect(nums)
		fmt.Printf("%#v -> %v\n", nums, correct)
		if correct {
			sum += nums[len(nums)/2]
		}
	}
	fmt.Printf("sum: %v\n", sum)

}
