package main

import (
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"log"
	"net"
	"os"
	"os/signal"

	"github.com/gdamore/tcell/v2"
	"github.com/gdamore/tcell/v2/terminfo"
	"golang.org/x/crypto/ssh"
)

type Config struct {
	Address             string `json:"address"`
	HostKeyFile         string `json:"host-key-file"`
	Password            string `json:"password"`
	WelcomeMessage      string `json:"welcome-message"`
	AuthorizedHostsFile string `json:"authorized-hosts-file"`
	Width               int    `json:"width"`
	Height              int    `json:"height"`
}

var messageLog *log.Logger

func main() {
	var config Config = Config{
		Address:             "localhost:2222",
		HostKeyFile:         fmt.Sprintf("%s/.ssh/id_rsa", os.Getenv("HOME")),
		AuthorizedHostsFile: fmt.Sprintf("%s/.ssh/known_hosts", os.Getenv("HOME")),
		Width:               80,
		Height:              40,
	}
	var configFile string = "./config.json"
	flag.StringVar(&configFile, "config", configFile, "path to a config file")
	flag.Parse()

	bytes, err := os.ReadFile(configFile)
	if err != nil {
		log.Fatal(err)
	}

	if err := json.Unmarshal(bytes, &config); err != nil {
		log.Fatal(err)
	}

	bytes, err = os.ReadFile(config.HostKeyFile)
	if err != nil {
		log.Fatal(err)
	}

	privateKey, err := ssh.ParsePrivateKey(bytes)
	if err != nil {
		log.Fatalf("failed to parse private key %s: %s", config.HostKeyFile, err.Error())
	}

	authorizedKeys := map[string]bool{}
	if len(config.AuthorizedHostsFile) > 0 {
		bytes, err := os.ReadFile(config.AuthorizedHostsFile)
		if err != nil {
			log.Fatalf("failed to parse authorized keys from %s: %s",
				config.AuthorizedHostsFile, err.Error())
		}

		for len(bytes) > 0 {
			publicKey, _, _, nextline, err := ssh.ParseAuthorizedKey(bytes)
			if err != nil {
				log.Fatalf("failed to parse authorized keys from %s: %s",
					config.AuthorizedHostsFile, err.Error())
			}

			authorizedKeys[string(publicKey.Marshal())] = true
			bytes = nextline
		}
	}

	sshconfig := &ssh.ServerConfig{}
	if len(authorizedKeys) > 0 {
		callback := func(conn ssh.ConnMetadata, key ssh.PublicKey) (*ssh.Permissions, error) {
			if !authorizedKeys[string(key.Marshal())] {
				return nil, errors.New("unknown key")
			}
			return nil, nil
		}
		sshconfig.PublicKeyCallback = callback
	}
	if len(config.Password) > 0 {
		callback := func(conn ssh.ConnMetadata, password []byte) (*ssh.Permissions, error) {
			if config.Password != string(password) {
				return nil, errors.New("wrong password")
			}
			return nil, nil
		}
		sshconfig.PasswordCallback = callback
	}

	sshconfig.AddHostKey(privateKey)

	log.Printf("[server] SSH Server listening on %s", config.Address)
	listener, err := net.Listen("tcp", config.Address)
	if err != nil {
		log.Fatal(err)
	}

	go func() {
		sigs := make(chan os.Signal, 16)
		signal.Notify(sigs, os.Interrupt)
		_ = <-sigs
		if err := listener.Close(); err != nil {
			log.Fatal(err)
		}
	}()

	game := NewGame(config.Width, config.Height)

	for {
		rawconn, err := listener.Accept()
		if err != nil {
			if errors.Is(err, net.ErrClosed) {
				break
			}
			log.Print(err)
			continue
		}

		go func() {
			conn, chans, reqs, err := ssh.NewServerConn(rawconn, sshconfig)
			if err != nil {
				log.Printf("ssh connection handshake failed: %s", err.Error())
				return
			}

			defer conn.Close()
			log.Printf("[server][user:%#v] connection established, addr=%#v, session-id=0x%s...",
				conn.User(), conn.LocalAddr().String(), hex.EncodeToString(conn.SessionID()[:6]))
			go ssh.DiscardRequests(reqs)
			for channelreq := range chans {
				if channelreq.ChannelType() != "session" {
					channelreq.Reject(ssh.UnknownChannelType, "Unknown Channel Type")
					continue
				}

				channel, reqs, err := channelreq.Accept()
				if err != nil {
					log.Print(err)
					conn.Close()
					break
				}

				user := &UserConnection{
					Channel:    channel,
					Connection: conn,
					User:       conn.User(),
				}
				log.Printf("[server][user:%#v] session established", conn.User())
				go func() {
					for req := range reqs {
						switch req.Type {
						case "shell":
							ti, err := terminfo.LookupTerminfo(user.Term)
							if err != nil {
								log.Printf("[user:%#v] shell: error: %s", user.User, err.Error())
								req.Reply(false, nil)
								continue
							}

							screen, err := tcell.NewTerminfoScreenFromTtyTerminfo(user, ti)
							if err != nil {
								log.Printf("[user:%#v] shell: error: %s", user.User, err.Error())
								req.Reply(false, nil)
								continue
							}

							if err := screen.Init(); err != nil {
								log.Printf("[user:%#v] shell: screen.Init error: %s",
									user.User, err.Error())
								req.Reply(false, nil)
								continue
							}

							go game.HandleConnection(user, screen)
							req.Reply(true, nil)
						case "pty-req":
							term, bytes, ok1 := parseString(req.Payload)
							width, bytes, ok2 := parseUint32(bytes)
							height, bytes, ok3 := parseUint32(bytes)
							if !ok1 || !ok2 || !ok3 {
								log.Printf("[user:%#v] pty-req: invalid request payload", user.User)
								req.Reply(false, nil)
								continue
							}
							user.Term = term
							user.TermWidth = int(width)
							user.TermHeight = int(height)
							req.Reply(true, nil)
							log.Printf("[server][user:%#v] terminal: %s, size: %dx%d",
								user.User, user.Term, user.TermWidth, user.TermHeight)
						case "window-change":
							width, bytes, ok1 := parseUint32(req.Payload)
							height, _, ok2 := parseUint32(bytes)
							if !ok1 || !ok2 {
								req.Reply(false, nil)
								continue
							}
							user.TermWidth = int(width)
							user.TermHeight = int(height)
							if user.ResizeCallback != nil {
								user.ResizeCallback()
							}
							req.Reply(true, nil)
						default:
							log.Printf("[server][user:%#v] Unknown req.Type %q with payload %q",
								user.User, req.Type, req.Payload)
						}
					}
				}()
			}
		}()
	}
}

// Implements the tcell.Pty/Tty type:
type UserConnection struct {
	ssh.Channel
	Connection     *ssh.ServerConn
	User           string
	Term           string
	TermWidth      int
	TermHeight     int
	ResizeCallback func()
}

func (uc *UserConnection) Start() error {
	return nil
}

func (uc *UserConnection) Stop() error {
	return nil
}

func (uc *UserConnection) Drain() error {
	return nil
}

func (uc *UserConnection) NotifyResize(callback func()) {
	uc.ResizeCallback = callback
}

func (uc *UserConnection) WindowSize() (int, int, error) {
	return uc.TermWidth, uc.TermHeight, nil
}

func parseString(in []byte) (string, []byte, bool) {
	length, tail, ok := parseUint32(in)
	if !ok || uint32(len(tail)) < length {
		return "", nil, false
	}

	return string(tail[:length]), tail[length:], true
}

func parseUint32(in []byte) (uint32, []byte, bool) {
	if len(in) < 4 {
		return 0, nil, false
	}
	return binary.BigEndian.Uint32(in), in[4:], true
}
