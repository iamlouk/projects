package main

import (
	"flag"
	"fmt"
	"log"

	"golang.org/x/net/websocket"
)

func main() {
	origin := "http://localhost/"
	url := "ws://localhost:12345/ws-echo"
	flag.StringVar(&url, "url", url, "server URL")
	flag.Parse()

	ws, err := websocket.Dial(url, "chat", origin)
	if err != nil {
		log.Fatal(err)
	}
	if _, err := ws.Write([]byte("hello, world!\n")); err != nil {
		log.Fatal(err)
	}
	msg := make([]byte, 512)
	var n int
	if n, err = ws.Read(msg); err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Received: %s.\n", msg[:n])
	if err := ws.Close(); err != nil {
		log.Fatal(err)
	}
}
