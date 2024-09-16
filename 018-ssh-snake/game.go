package main

import (
	"fmt"
	"log"
	"math/rand"
	"os"
	"sync"

	"github.com/gdamore/tcell/v2"
)

type Game struct {
	width, height int
	lock sync.Mutex
	players map[string]*Player
	colors []tcell.Color
}

type Pos struct { X, Y int }

func NewGame(width, height int) *Game {
	return &Game{
		width: width,
		height: height,
		players: make(map[string]*Player),
		colors: []tcell.Color{
			tcell.GetColor("green"),
			tcell.GetColor("red"),
			tcell.GetColor("lime"),
			tcell.GetColor("yellow"),
			tcell.GetColor("blue"),
			tcell.GetColor("teal"),
		},
	}
}

type PlayerMove struct {
	Pos   Pos
	Color tcell.Color
	User  string
}

type Player struct {
	Username string
	Color tcell.Color
	Head Pos
	Tail []Pos
	Moves chan *PlayerMove
}

func (game *Game) HandleConnection(user *UserConnection, screen tcell.Screen) {
	events := make(chan tcell.Event)
	quit := make(chan struct{})
	logger := log.New(os.Stderr, fmt.Sprintf("[server][user=%#v] ", user.User), log.Ltime | log.Ldate | log.Lmsgprefix)
	defer close(quit)
	defer screen.Fini()
	defer user.Close()
	go screen.ChannelEvents(events, quit)

	game.lock.Lock()
	if game.players[user.User] != nil || len(game.colors) == 0 {
		logger.Printf("username already taken or game full")
		user.Write([]byte("username already taken or game full"))
		game.lock.Unlock()
		return
	}
	screen.Clear()
	for _, p := range game.players {
		screen.SetContent(p.Head.X, p.Head.Y, 'O', nil, tcell.StyleDefault.Foreground(p.Color))
		for _, pos := range p.Tail {
			screen.SetContent(pos.X, pos.Y, 'O', nil, tcell.StyleDefault.Foreground(p.Color))
		}
	}
	screen.Show()
	player := &Player{
		Username: user.User,
		Head: Pos{
			X: rand.Int() % game.width,
			Y: rand.Int() % game.height,
		},
		Tail: make([]Pos, 0, 64),
		Color: game.colors[0],
		Moves: make(chan *PlayerMove, 16),
	}
	game.players[player.Username] = player
	game.colors = game.colors[1:]
	game.lock.Unlock()

	logger.Printf("game successfully joined, color=%#x", player.Color.Hex())
	game.Move(player)
	for {
		select {
		case move := <-player.Moves:
			screen.SetContent(move.Pos.X, move.Pos.Y, 'O', nil, tcell.StyleDefault.Foreground(move.Color))
			screen.Show()
		case event := <-events:
			switch evt := event.(type) {
			case *tcell.EventError:
				logger.Printf("error event: %s", evt.Error())
			case *tcell.EventKey:
				switch evt.Key() {
				case tcell.KeyUp:
					player.Up(game.width, game.height)
					game.Move(player)
				case tcell.KeyDown:
					player.Down(game.width, game.height)
					game.Move(player)
				case tcell.KeyLeft:
					player.Left(game.width, game.height)
					game.Move(player)
				case tcell.KeyRight:
					player.Right(game.width, game.height)
					game.Move(player)
				case tcell.KeyEscape:
					return
				}
			}
		}
	}
}

func (game *Game) Move(player *Player) {
	move := &PlayerMove{
		Pos: player.Head,
		Color: player.Color,
		User: player.Username,
	}

	game.lock.Lock()
	defer game.lock.Unlock()
	for _, p := range game.players {
		p.Moves <- move
	}
}

func (p *Player) Up(w, h int) {
	p.Tail = append(p.Tail, p.Head)
	p.Head.Y -= 1
	if p.Head.Y < 0 {
		p.Head.Y = h - 1
	}
}

func (p *Player) Down(w, h int) {
	p.Tail = append(p.Tail, p.Head)
	p.Head.Y += 1
	if p.Head.Y >= h {
		p.Head.Y = 0
	}
}

func (p *Player) Left(w, h int) {
	p.Tail = append(p.Tail, p.Head)
	p.Head.X -= 1
	if p.Head.X < 0 {
		p.Head.X = w - 1
	}
}

func (p *Player) Right(w, h int) {
	p.Tail = append(p.Tail, p.Head)
	p.Head.X += 1
	if p.Head.X >= w {
		p.Head.X = 0
	}
}

