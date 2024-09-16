package main

import (
	"bufio"
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"strings"
	"time"

	"golang.org/x/net/websocket"
)

func main() {
	log.SetPrefix("ws-chat.go:")
	origin := "http://localhost/"
	url := "ws://localhost:12345/chat"
	username := os.Getenv("USER")
	flag.StringVar(&url, "url", url, "Chat Server URL")
	flag.StringVar(&username, "username", username, "Username")
	flag.Parse()
	ws, err := websocket.Dial(url, "chat", origin)
	if err != nil {
		log.Fatal(err)
	}

	sigchan := make(chan os.Signal, 1)
	signal.Notify(sigchan, os.Interrupt)
	go func() {
		for sig := range sigchan {
			if err := ws.Close(); err != nil {
				log.Fatal(err)
			}
			log.Printf("gracefull exit because of signal %s", sig.String())
			os.Exit(0)
		}
	}()

	if _, err := ws.Write([]byte("User <" + username + "> joined!")); err != nil {
		log.Fatal(err)
	}

	go func() {
		buf := make([]byte, 4096)
		for {
			n, err := ws.Read(buf)
			if err != nil {
				log.Fatal(err)
			}
			fmt.Println(string(buf[0:n]))
		}
	}()

	stdin := bufio.NewReader(os.Stdin)
	for {
		msg, err := stdin.ReadString('\n')
		if err != nil {
			log.Fatal(err)
		}
		msg = "<" + username + ">: " + strings.TrimSpace(msg)
		if _, err := ws.Write([]byte(msg)); err != nil {
			log.Fatal(err)
		}

		time.Sleep(100 * time.Millisecond)
	}
}
