package main

import (
	"fmt"
	"log"
	"math/rand"
	"os"
	"strconv"
	"sync"
	"time"

	"github.com/gdamore/tcell/v2"
)

type Pos struct { X, Y int }

type Player struct {
	Username  string
	Screen    tcell.Screen
	Color     tcell.Color
	Head      Pos
	Direction Pos
	Tail      []Pos
	Actions   chan Action
	Log       *log.Logger
	Over      bool
}

func (player *Player) Move(game *Game) {
	if player.Direction.X != 0 || player.Direction.Y != 0 {
		player.Tail = append(player.Tail, player.Head)
	}
	player.Head.X += player.Direction.X
	player.Head.Y += player.Direction.Y

	if player.Head.X < 0 {
		player.Head.X = game.Width - 1
	}
	if player.Head.X >= game.Width {
		player.Head.X = 0
	}
	if player.Head.Y < 0 {
		player.Head.Y = game.Height - 1
	}
	if player.Head.Y >= game.Height {
		player.Head.Y = 0
	}

	game.Grid[player.Head.Y*game.Width+player.Head.X] = player.Color
}

type Action interface{}

type ActionPlayerJoin struct {
	Player    *Player
	Others    []*Player
	Positions [][]Pos
}

type ActionPlayerLeave struct {
	Player *Player
}

type ActionPlayerDirectionChange struct {
	Player       *Player
	NewDirection Pos
}

type ActionUpdate struct {
	Time    time.Time
	Updates []PositionUpdate
}

type PositionUpdate struct {
	Player  *Player
	Pos     Pos
	PrevPos Pos
}

type Game struct {
	GameId        string
	Width, Height int
	Grid          []tcell.Color
	Lock          sync.Mutex
	Players       map[string]*Player
	Colors        []tcell.Color
	Actions       chan Action
	Ticker        *time.Ticker
	Over          bool
	Log           *log.Logger
}

func (game *Game) Transform(pos Pos) (int, int) {
	return pos.X + 1, pos.Y + 2
}

func (game *Game) SendAction(action Action) {
	for _, player := range game.Players {
		if !player.Over {
			player.Actions <- action
		}
	}
}

func NewGame(width, height int) *Game {
	gameId := strconv.FormatUint(uint64(rand.Uint32()), 16)
	game := &Game{
		GameId:  gameId,
		Width:   width,
		Height:  height,
		Grid:    make([]tcell.Color, width*height),
		Players: make(map[string]*Player),
		Colors: []tcell.Color{
			tcell.GetColor("green").TrueColor(),
			tcell.GetColor("red").TrueColor(),
			tcell.GetColor("lime").TrueColor(),
			tcell.GetColor("yellow").TrueColor(),
			tcell.GetColor("blue").TrueColor(),
			tcell.GetColor("teal").TrueColor(),
		},
		Actions: make(chan Action, 128),
		Ticker:  time.NewTicker(200 * time.Millisecond),
		Over:    false,
		Log: log.New(
			os.Stderr,
			fmt.Sprintf("[game:%s] ", gameId),
			log.Ldate|log.Ltime|log.Lmsgprefix),
	}

	go func() {
		for !game.Over {
			select {
			case action, ok := <-game.Actions:
				if !ok {
					return
				}
				switch act := action.(type) {
				case *ActionPlayerJoin:
					player := act.Player
					if len(game.Colors) == 0 || game.Players[player.Username] != nil {
						game.Log.Print("could not join: too many players or username taken")
						close(player.Actions)
						continue
					}

					i, maxtries := 0, 250
					for i < maxtries {
						player.Head.X = rand.Int() % game.Width
						player.Head.Y = rand.Int() % game.Height
						col := game.Grid[player.Head.Y*game.Width+player.Head.X]
						if col == 0 {
							break
						}
						i += 1
					}
					if i == maxtries {
						game.Log.Print("could not join: grid filled")
						close(player.Actions)
						continue
					}

					join := &ActionPlayerJoin{
						Player:    player,
						Others:    make([]*Player, 0, len(game.Players)),
						Positions: make([][]Pos, 0, len(game.Players)),
					}
					for _, p := range game.Players {
						join.Others = append(join.Others, p)
						positions := make([]Pos, 0, 1+len(p.Tail))
						join.Positions = append(join.Positions, append(positions, p.Tail...))
					}

					player.Color = game.Colors[0]
					player.Tail = append(player.Tail, player.Head)
					game.Players[player.Username] = player
					game.Colors = game.Colors[1:]
					game.SendAction(join)
				case *ActionPlayerLeave:
					player := act.Player
					player.Over = true
					close(player.Actions)
					game.Log.Printf("player %#v left the game", player.Username)
				case *ActionPlayerDirectionChange:
					player := act.Player
					player.Direction = act.NewDirection
				default:
					panic("invalid action")
				}
			case timestamp, ok := <-game.Ticker.C:
				if !ok {
					return
				}

				update := &ActionUpdate{
					Time:    timestamp,
					Updates: make([]PositionUpdate, 0, len(game.Players)),
				}

				for _, player := range game.Players {
					if !game.Over {
						prevPos := player.Head
						player.Move(game)
						update.Updates = append(update.Updates, PositionUpdate{
							Player:  player,
							Pos:     player.Head,
							PrevPos: prevPos,
						})
					}
				}

				game.SendAction(update)
			}
		}
	}()

	game.Log.Print("Game Started!")
	return game
}

func (game *Game) End() {
	close(game.Actions)
	game.Ticker.Stop()
	game.Over = true
	game.Log.Print("Game Over!")
}

func (game *Game) HandleConnection(user *UserConnection, screen tcell.Screen) {
	events := make(chan tcell.Event)
	quit := make(chan struct{})
	go screen.ChannelEvents(events, quit)

	me := &Player{
		Username: user.User,
		Screen:   screen,
		Tail:     make([]Pos, 0, 128),
		Actions:  make(chan Action, 128),
		Log: log.New(
			game.Log.Writer(),
			fmt.Sprintf("%s[user:%#v] ",
				game.Log.Prefix()[:len(game.Log.Prefix())-1],
				user.User),
			game.Log.Flags()),
	}

	defer func() {
		if me.Color != 0 {
			me.Log.Print("leaving game")
			game.Actions <- &ActionPlayerLeave{Player: me}
		}
		screen.Fini()
		user.Close()
		user.Connection.Close()
		close(quit)
	}()

	bstyle := tcell.StyleDefault
	screen.Clear()
	screen.SetContent(0, 1, '+', nil, bstyle)
	screen.SetContent(1+game.Width, 1, '+', nil, bstyle)
	screen.SetContent(0, 2+game.Height, '+', nil, bstyle)
	screen.SetContent(1+game.Width, 2+game.Height, '+', nil, bstyle)
	for i := 0; i < game.Width; i++ {
		screen.SetContent(1+i, 1, '-', nil, bstyle)
		screen.SetContent(1+i, 2+game.Height, '+', nil, bstyle)
	}
	for i := 0; i < game.Height; i++ {
		screen.SetContent(0, 2+i, '|', nil, bstyle)
		screen.SetContent(1+game.Width, 2+i, '|', nil, bstyle)
	}

	screen.Show()
	me.Log.Print("trying to join the game...")
	game.Actions <- &ActionPlayerJoin{Player: me}
	response, ok := <-me.Actions
	if !ok {
		me.Log.Print("me.Actions channel closed")
		return
	}

	join, ok := response.(*ActionPlayerJoin)
	if !ok || join.Player != me {
		me.Log.Print("unexpected response type to join")
		return
	}

	x, y := game.Transform(me.Head)
	screen.SetContent(x, y, 'Ö', nil, tcell.StyleDefault.Foreground(me.Color))

	for i := 0; i < len(join.Others); i++ {
		color := join.Others[i].Color
		style := tcell.StyleDefault.Foreground(color)
		positions := join.Positions[i]
		x, y := game.Transform(positions[0])
		screen.SetContent(x, y, 'Ö', nil, style)
		for i := 1; i < len(positions); i++ {
			x, y := game.Transform(positions[i])
			screen.SetContent(x, y, 'O', nil, style)
		}
	}

	screen.Show()
	me.Log.Printf("game joined!")

	for {
		select {
		case event, ok := <-events:
			if !ok {
				return
			}
			switch evt := event.(type) {
			case *tcell.EventError:
				log.Printf("tcell error: %s", evt.Error())
				return
			case *tcell.EventKey:
				switch evt.Key() {
				case tcell.KeyUp:
					game.Actions <- &ActionPlayerDirectionChange{
						Player:       me,
						NewDirection: Pos{X: 0, Y: -1},
					}
				case tcell.KeyDown:
					game.Actions <- &ActionPlayerDirectionChange{
						Player:       me,
						NewDirection: Pos{X: 0, Y: 1},
					}
				case tcell.KeyLeft:
					game.Actions <- &ActionPlayerDirectionChange{
						Player:       me,
						NewDirection: Pos{X: -1, Y: 0},
					}
				case tcell.KeyRight:
					game.Actions <- &ActionPlayerDirectionChange{
						Player:       me,
						NewDirection: Pos{X: 1, Y: 0},
					}
				case tcell.KeyEscape:
					return
				}
			}
		case action, ok := <-me.Actions:
			if !ok {
				return
			}
			switch act := action.(type) {
			case *ActionPlayerJoin:
				x, y := game.Transform(act.Player.Head)
				style := tcell.StyleDefault.Foreground(act.Player.Color)
				screen.SetContent(x, y, 'Ö', nil, style)
				screen.Show()
			case *ActionUpdate:
				for _, peer := range act.Updates {
					var x, y int
					style := tcell.StyleDefault.Foreground(peer.Player.Color)
					x, y = game.Transform(peer.PrevPos)
					screen.SetContent(x, y, 'O', nil, style)
					x, y = game.Transform(peer.Pos)
					screen.SetContent(x, y, 'Ö', nil, style)
				}
				screen.Show()
			default:
				panic("invalid action")
			}
		}
	}
}

