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

type Game struct {
	GameId        string
	Width, Height int
	Grid          []tcell.Color
	Lock          sync.Mutex
	Players       map[string]*Player
	Colors        []tcell.Color
	BerryColor    tcell.Color
	Berries       map[Pos]Pos
	Actions       chan Action
	Ticker        *time.Ticker
	Over          bool
	Log           *log.Logger
}

type Player struct {
	Username  string
	Screen    tcell.Screen
	Color     tcell.Color
	Snake     Snake
	Actions   chan Action
	Log       *log.Logger
	Over      bool
}

func (player *Player) Move(game *Game) (Pos, Pos) {
	newpos, oldpos := player.Snake.Move(game.Width, game.Height)
	game.Grid[newpos.Y*game.Width+newpos.X] = player.Color
	game.Grid[oldpos.Y*game.Width+oldpos.X] = 0
	return newpos, oldpos
}

type Action interface{}

type ActionPlayerJoin struct {
	Player    *Player
	Others    []*Player
	Positions [][]Pos
	Berries   []Pos
}

type ActionPlayerLeave struct {
	Player *Player
}

type ActionPlayerDirectionChange struct {
	Player       *Player
	NewDirection Pos
}

type ActionUpdate struct {
	Time       time.Time
	Updates    []PositionUpdate
	NewBerries []Pos
}

type PositionUpdate struct {
	Player    *Player
	Head      Pos
	PrevHead  Pos
	Tail      Pos
	ClearTail bool
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
			tcell.GetColor("blue").TrueColor(),
			tcell.GetColor("teal").TrueColor(),
		},
		BerryColor: tcell.GetColor("yellow").TrueColor(),
		Berries: make(map[Pos]Pos),
		Actions: make(chan Action, 128),
		Ticker:  time.NewTicker(200 * time.Millisecond),
		Over:    false,
		Log: log.New(
			os.Stderr,
			fmt.Sprintf("[game:%s] ", gameId),
			log.Ldate|log.Ltime|log.Lmsgprefix),
	}

	for i := 0; i < 3; i++ {
		game.AddBerry()
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

					x, y, i, maxtries := 0, 0, 0, 250
					for i < maxtries {
						x = rand.Int() % game.Width
						y = rand.Int() % game.Height
						col := game.Grid[y*game.Width+x]
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

					player.Snake.Init(x, y, 6)

					join := &ActionPlayerJoin{
						Player:    player,
						Others:    make([]*Player, 0, len(game.Players)),
						Positions: make([][]Pos, 0, len(game.Players)),
					}
					for _, p := range game.Players {
						join.Others = append(join.Others, p)
						join.Positions = append(join.Positions, p.Snake.ToArray())
					}
					for pos := range game.Berries {
						join.Berries = append(join.Berries, pos)
					}

					player.Color = game.Colors[0]
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
					player.Snake.Direction = act.NewDirection
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
						prevhead := player.Snake.Head.Pos
						newpos, tailpos := player.Move(game)
						update.Updates = append(update.Updates, PositionUpdate{
							Player:    player,
							Head:      newpos,
							PrevHead:  prevhead,
							Tail:      tailpos,
							ClearTail: true,
						})

						if pos, ok := game.Berries[newpos]; ok {
							player.Snake.Grow()
							delete(game.Berries, pos);
							berry := game.AddBerry()
							update.NewBerries = append(update.NewBerries, berry)
						}
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

func (game *Game) AddBerry() Pos {
	for {
		pos := Pos{
			X: rand.Int() % game.Width,
			Y: rand.Int() % game.Height,
		}
		if _, ok := game.Berries[pos]; ok {
			goto tryagain;
		}
		for _, p := range game.Players {
			if p.Snake.Contains(pos) {
				goto tryagain;
			}
		}

		game.Grid[pos.Y*game.Width+pos.X] = game.BerryColor
		game.Berries[pos] = pos
		return pos
	tryagain:
	}
}

func (game *Game) HandleConnection(user *UserConnection, screen tcell.Screen) {
	events := make(chan tcell.Event)
	quit := make(chan struct{})
	go screen.ChannelEvents(events, quit)

	me := &Player{
		Username: user.User,
		Screen:   screen,
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

	screen.Clear()
	style := tcell.StyleDefault
	screen.SetContent(0, 1, '+', nil, style)
	screen.SetContent(1+game.Width, 1, '+', nil, style)
	screen.SetContent(0, 2+game.Height, '+', nil, style)
	screen.SetContent(1+game.Width, 2+game.Height, '+', nil, style)
	for i := 0; i < game.Width; i++ {
		screen.SetContent(1+i, 1, '-', nil, style)
		screen.SetContent(1+i, 2+game.Height, '+', nil, style)
	}
	for i := 0; i < game.Height; i++ {
		screen.SetContent(0, 2+i, '|', nil, style)
		screen.SetContent(1+game.Width, 2+i, '|', nil, style)
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

	x, y := game.Transform(me.Snake.Head.Pos)
	screen.SetContent(x, y, 'Ö', nil, tcell.StyleDefault.Foreground(me.Color))

	style = tcell.StyleDefault.Foreground(me.Color)
	for i, r := range fmt.Sprintf("Hi %s!", me.Username) {
		screen.SetContent(i, 0, r, nil, style)
	}

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

	style = tcell.StyleDefault.Foreground(game.BerryColor)
	for _, pos := range join.Berries {
		x, y := game.Transform(pos)
		screen.SetContent(x, y, '+', nil, style)
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
				x, y := game.Transform(act.Player.Snake.Head.Pos)
				style := tcell.StyleDefault.Foreground(act.Player.Color)
				screen.SetContent(x, y, 'Ö', nil, style)
				screen.Show()
			case *ActionUpdate:
				for _, peer := range act.Updates {
					var x, y int
					style := tcell.StyleDefault.Foreground(peer.Player.Color)
					x, y = game.Transform(peer.PrevHead)
					screen.SetContent(x, y, 'O', nil, style)
					x, y = game.Transform(peer.Head)
					screen.SetContent(x, y, 'Ö', nil, style)
					if peer.ClearTail && peer.Tail != peer.Head {
						x, y = game.Transform(peer.Tail)
						screen.SetContent(x, y, ' ', nil, tcell.StyleDefault)
					}
				}
				for _, pos := range act.NewBerries {
					style := tcell.StyleDefault.Foreground(game.BerryColor)
					x, y = game.Transform(pos)
					screen.SetContent(x, y, '+', nil, style)
				}
				screen.Show()
			default:
				panic("invalid action")
			}
		}
	}
}

