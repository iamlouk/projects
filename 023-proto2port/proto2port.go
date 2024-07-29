package proto2port

import (
	"bufio"
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"log"
	"net"
	"os"
	"regexp"

	"gopkg.in/yaml.v3"
)

type Service struct {
	Name     string `yaml:"name"`
	RawRegex string `yaml:"match"`
	DstPort  uint16 `yaml:"dstport"`
	Regex    *regexp.Regexp
}

type Config struct {
	Port             uint16     `yaml:"port"`
	Services         []*Service `yaml:"protocols"`
	FallbackResponse string     `yaml:"fallback-response"`
}

func main() {
	configFile := flag.String("config", "config.yaml", "YAML (or JSON) config file path")
	flag.Parse()
	configBytes, err := os.ReadFile(*configFile)
	if err != nil {
		log.Fatalf("failed to read %s: %s", *configFile, err.Error())
	}

	var config Config
	if err := yaml.Unmarshal(configBytes, &config); err != nil {
		log.Fatalf("failed to parse %s: %s", *configFile, err.Error())
	}

	for _, service := range config.Services {
		service.Regex = regexp.MustCompile(service.RawRegex)
	}

	listener, err := net.Listen("tcp", fmt.Sprintf("::%d", config.Port))
	if err != nil {
		log.Fatal(err.Error())
	}
	defer listener.Close()
	ctx := context.Background()
	for {
		/* TODO: Add sigint handler etc. and create a context or so for
		   proper shutdown logic. */
		conn, err := listener.Accept()
		if err != nil {
			if errors.Is(err, net.ErrClosed) {
				break
			}

			log.Printf("accept failed: %s", err.Error())
			continue
		}

		go handleConnection(&config, ctx, conn)
	}
}

func handleConnection(config *Config, ctx context.Context, conn net.Conn) {
	clientReader := bufio.NewReader(conn)
	clientWriter := bufio.NewWriter(conn)
	line, err := clientReader.ReadBytes('\n')
	if err != nil {
		log.Printf("read of first line failed: %s", err.Error())
		return
	}

	var service *Service = nil
	for _, s := range config.Services {
		if s.Regex.Match(line) {
			service = s
			break
		}
	}

	if service == nil {
		clientWriter.Write([]byte(config.FallbackResponse))
		clientWriter.Flush()
		conn.Close()
		return
	}

	sconn, err := net.Dial("tcp", fmt.Sprintf("::%d", service.DstPort))
	if err != nil {
		log.Printf("dialing %s ('::%d'): %s", service.Name, service.DstPort, err.Error())
		return
	}

	serviceReader := bufio.NewReader(sconn)
	serviceWriter := bufio.NewWriter(sconn)
	if _, err := serviceWriter.Write(line); err != nil {
		log.Printf("failed to forward first line: %s", err.Error())
		return
	}

	go io.Copy(serviceWriter, clientReader)
	go io.Copy(clientWriter, serviceReader)
}
