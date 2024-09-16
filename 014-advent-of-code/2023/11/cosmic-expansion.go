package main

import (
	"bufio"
	"fmt"
	"log"
	"math"
	"os"
)



func main() {
	fmt.Println("Hello World!")

	rows := make([][]int32, 0)
	stdin := bufio.NewScanner(os.Stdin)
	for stdin.Scan() {
		row := make([]int32, 0)
		line := stdin.Text()
		// fmt.Printf("%#v\n", line)

		anygalaxy := false
		for _, c := range line {
			if c == '.' {
				row = append(row, 0)
			} else if c == '#' {
				anygalaxy = true
				row = append(row, 1)
			} else {
				log.Fatalf("unexpected: %#v", c)
			}
		}

		rows = append(rows, row)

		if !anygalaxy {
			rows = append(rows, append([]int32{}, row...))
		}

	}
	if err := stdin.Err(); err != nil {
		log.Fatal(err)
	}

	numcols := len(rows[0])
	for col := 0; col < numcols; col++ {
		anygalaxy := false
		for row := 0; row < len(rows); row++ {
			if rows[row][col] == 1 {
				anygalaxy = true
				break
			}
		}

		if !anygalaxy {
			for row := 0; row < len(rows); row++ {
				nrow := make([]int32, 0, len(rows[row]) + 1)
				nrow = append(nrow, rows[row][0:col]...)
				nrow = append(nrow, 0)
				nrow = append(nrow, rows[row][col:]...)
				rows[row] = nrow
			}
			numcols++
			col++
		}
	}

	galaxies := make([][2]int, 0)
	for y, row := range rows {
		for x, hasgalaxy := range row {
			if hasgalaxy != 0 {
				galaxies = append(galaxies, [2]int{ y, x })
			}
			// fmt.Printf("%d", hasgalaxy)
		}
		// fmt.Println()
	}

	sum := 0
	for i, g1 := range galaxies {
		for j, g2 := range galaxies {
			if i <= j {
				continue
			}

			dy := math.Abs(float64(g1[0] - g2[0]))
			dx := math.Abs(float64(g1[1] - g2[1]))
			d := int(dx + dy)
			sum += d

		}
	}
	fmt.Printf("Sum: %d\n", sum)
}

