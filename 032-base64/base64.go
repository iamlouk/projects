package main

import (
	"errors"
	"fmt"
	"io"
	"os"
)

/* Arrays cannot be constants in Go. */
var Base64Chars [64]uint8 = [64]uint8{'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '+', '/'}

func Base64Encode(input []uint8, buf []uint8, last bool, output io.Writer) (int, error) {
	chunks, rem := len(input)/3, len(input)%3
	buf = buf[0:0]
	for c := 0; c < chunks; c++ {
		b1, b2, b3 := input[c*3+0], input[c*3+1], input[c*3+2]
		buf = append(buf, Base64Chars[(b1>>2)&0x3f])
		buf = append(buf, Base64Chars[((b1<<4)&0x30)|((b2>>4)&0x0f)])
		buf = append(buf, Base64Chars[((b2<<2)&0x3c)|((b3>>6)&0x03)])
		buf = append(buf, Base64Chars[(b3&0x3f)])
	}

	if !last {
		_, err := output.Write(buf)
		return chunks * 3, err
	}

	if rem == 1 {
		b1 := input[chunks*3+0]
		buf = append(buf, Base64Chars[(b1>>2)&0x3f])
		buf = append(buf, Base64Chars[(b1<<4)&0x30])
		buf = append(buf, '=')
		buf = append(buf, '=')
	} else if rem == 2 {
		b1, b2 := input[chunks*3+0], input[chunks*3+1]
		buf = append(buf, Base64Chars[(b1>>2)&0x3f])
		buf = append(buf, Base64Chars[((b1<<4)&0x30)|((b2>>4)&0x0f)])
		buf = append(buf, Base64Chars[(b2<<2)&0x3c])
		buf = append(buf, '=')
	}

	_, err := output.Write(buf)
	return chunks*3 + rem, err
}

func main() {
	bufcap := 4096
	buf := make([]uint8, bufcap)
	tmpbuf := make([]uint8, bufcap+bufcap/2)
	startoffset := 0
	for {
		done := false
		n, err := os.Stdin.Read(buf[startoffset:])
		if err != nil {
			if !errors.Is(err, io.EOF) {
				fmt.Fprintf(os.Stderr, "error while reading from stdin: %s", err.Error())
				os.Exit(1)
				return
			}
			done = true
		}

		m, err := Base64Encode(buf[0:startoffset+n], tmpbuf, done, os.Stdout)
		if err != nil {
			fmt.Fprintf(os.Stderr, "error while writing to stdout: %s", err.Error())
			os.Exit(1)
			return
		}

		if done {
			os.Stdout.Close()
			break
		}

		startoffset = (startoffset + n) - m
		for i := 0; i < startoffset; i++ {
			buf[i] = buf[m+i]
		}
	}
}
