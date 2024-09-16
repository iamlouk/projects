package main

import (
	"bufio"
	"crypto/rand"
	"crypto/sha1"
	"encoding/base64"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"time"
)

// TODO: Proper error handling.

type WebSocketConn struct {
	HttpReq *http.Request
	Conn    net.Conn
	BufRW   *bufio.ReadWriter
	SecKey  [16]byte
}

type WebSocketFrame struct {
	WSC        *WebSocketConn
	Fin        bool
	Masked     bool
	OpCode     uint8
	PayloadLen uint
	MaskingKey [4]byte
	Payload    []byte
}

const (
	FrameContinuation    uint8 = 0
	FrameText            uint8 = 1
	FrameBinary          uint8 = 2
	FrameConnectionClose       = 8
	FramePing                  = 9
	FramePong                  = 10
)

func NewWebSocketConnection(res http.ResponseWriter, req *http.Request) (*WebSocketConn, error) {
	log.Printf("log: %#v", req)
	hj, ok := res.(http.Hijacker)
	if !ok || req.Header.Get("Upgrade") != "websocket" ||
		req.Header.Get("Connection") != "Upgrade" {
		res.WriteHeader(http.StatusBadRequest)
		res.Write([]byte("Expected a websocket connection upgrade req.!"))
		return nil, errors.New("not a websocket upgrade req.")
	}

	if req.Header.Get("Sec-WebSocket-Version") != "13" {
		res.WriteHeader(http.StatusBadRequest)
		res.Write([]byte("Expected a websocket connection upgrade req. for version 13!"))
		return nil, errors.New("only version 13 of the websocket protocol is supported.")
	}

	conn, bufrw, err := hj.Hijack()
	if err != nil {
		res.WriteHeader(http.StatusInternalServerError)
		res.Write([]byte(err.Error()))
		return nil, err
	}

	seckey, err := base64.StdEncoding.DecodeString(req.Header.Get("Sec-WebSocket-Key"))
	if err != nil || len(seckey) != 16 {
		return nil, fmt.Errorf("'Sec-WebSocket-Key' header field: %s",
			req.Header.Get("Sec-WebSocket-Key"))
	}

	wsc := &WebSocketConn{
		HttpReq: req,
		Conn:    conn,
		BufRW:   bufrw,
	}
	copy(wsc.SecKey[:], seckey)
	hasher := sha1.New()
	hasher.Write([]byte(req.Header.Get("Sec-WebSocket-Key")))
	hasher.Write([]byte("258EAFA5-E914-47DA-95CA-C5AB0DC85B11"))
	acceptKey := base64.StdEncoding.EncodeToString(hasher.Sum(nil))
	bufrw.WriteString(req.Proto)
	bufrw.WriteString(" 101 Switching Protocols\r\n")
	bufrw.WriteString("Upgrade: websocket\r\n")
	bufrw.WriteString("Connection: Upgrade\r\n")
	bufrw.WriteString("Sec-WebSocket-Version: 13\r\n")
	bufrw.WriteString("Sec-WebSocket-Accept: ")
	bufrw.WriteString(acceptKey)
	bufrw.WriteString("\r\n\r\n")
	if err := bufrw.Flush(); err != nil {
		return nil, err
	}

	go func() {
		for {
			frame, err := wsc.DecodeFrame()
			if err != nil {
				panic(err)
			}
			log.Printf("frame: %#v", frame)

			if frame.OpCode == FramePing {
				pong := &WebSocketFrame{
					WSC:    wsc,
					Fin:    true,
					OpCode: FramePong,
				}
				if err := wsc.SendFrame(pong); err != nil {
					log.Fatalf("failed to send pong frame: %s", err.Error())
				}
				continue
			}

			if frame.OpCode == FrameConnectionClose {
				log.Print("Disconnection Frame!")
				wsc.Conn.Close()
				return
			}

			if frame.OpCode == FrameContinuation || !frame.Fin {
				log.Fatalf("unimplemented: Continuation/non-Fin frame: %#v", frame)
				return
			}

			if (frame.OpCode == FrameText || frame.OpCode == FrameBinary) && frame.Fin {
				echo := &WebSocketFrame{
					WSC:     wsc,
					Fin:     true,
					OpCode:  frame.OpCode,
					Payload: frame.Payload,
				}
				if err := wsc.SendFrame(echo); err != nil {
					log.Fatalf("failed to send frame: %s", err.Error())
				}
				log.Printf("response frame: %#v", echo)
				continue
			}

			log.Fatalf("unimplemented: frame: %#v", frame)
		}
	}()

	return wsc, nil
}

func (wsc *WebSocketConn) DecodeFrame() (*WebSocketFrame, error) {
	b0, err := wsc.BufRW.ReadByte()
	if err != nil {
		return nil, err
	}
	b1, err := wsc.BufRW.ReadByte()
	if err != nil {
		return nil, err
	}

	frame := &WebSocketFrame{
		WSC:        wsc,
		Fin:        b0&0x80 != 0,
		OpCode:     b0 & 0x7f,
		Masked:     b1&0x80 != 0,
		PayloadLen: uint(b1 & 0x7f),
	}
	if (3 <= frame.OpCode && frame.OpCode <= 7) || (frame.OpCode >= 10) {
		return nil, fmt.Errorf("invalid WebSocket frame opcode: %#v", frame.OpCode)
	}
	if frame.PayloadLen >= 126 {
		var buf [8]byte
		if frame.PayloadLen == 126 {
			if _, err := io.ReadFull(wsc.BufRW, buf[0:2]); err != nil {
				return nil, err
			}
			frame.PayloadLen = (uint(buf[0]) << 8) | uint(buf[1])
		} else {
			if _, err := io.ReadFull(wsc.BufRW, buf[0:8]); err != nil {
				return nil, err
			}
			frame.PayloadLen = uint(binary.BigEndian.Uint64(buf[:]))
		}
	}
	if frame.Masked {
		if _, err := io.ReadFull(wsc.BufRW, frame.MaskingKey[:]); err != nil {
			return nil, err
		}
	}
	frame.Payload = make([]byte, frame.PayloadLen)
	if _, err := io.ReadFull(wsc.BufRW, frame.Payload); err != nil {
		return nil, err
	}
	if frame.Masked {
		for i := uint(0); i < frame.PayloadLen; i++ {
			frame.Payload[i] ^= frame.MaskingKey[i%4]
		}
	}

	return frame, nil
}

func (wsc *WebSocketConn) SendFrame(frame *WebSocketFrame) error {
	frame.PayloadLen = uint(len(frame.Payload))
	buf := make([]byte, 0, 16+frame.PayloadLen)
	b0 := byte(frame.OpCode)
	if frame.Fin {
		b0 |= 1 << 7
	}
	buf = append(buf, b0)
	masked := byte(0)
	if frame.Masked {
		if frame.MaskingKey == [4]byte{0, 0, 0, 0} {
			if _, err := rand.Read(frame.MaskingKey[:]); err != nil {
				return err
			}
		}
		masked |= 1 << 7
	}
	if frame.PayloadLen < 126 {
		buf = append(buf, masked|byte(frame.PayloadLen))
	} else if frame.PayloadLen < 65536 {
		buf = append(buf, masked|byte(126))
		buf = append(buf, byte((frame.PayloadLen&0xff00)>>8))
		buf = append(buf, byte(frame.PayloadLen&0xff))
	} else {
		buf = append(buf, masked|byte(127))
		buf = binary.BigEndian.AppendUint64(buf, uint64(frame.PayloadLen))
	}
	if frame.Masked {
		buf = append(buf, frame.MaskingKey[:]...)
		for i := range frame.PayloadLen {
			buf = append(buf, frame.Payload[i]^frame.MaskingKey[i%4])
		}
		if _, err := wsc.BufRW.Write(buf); err != nil {
			return err
		}
	} else {
		if _, err := wsc.BufRW.Write(buf); err != nil {
			return err
		}
		if _, err := wsc.BufRW.Write(frame.Payload[0:frame.PayloadLen]); err != nil {
			return err
		}
	}

	return wsc.BufRW.Flush()
}

func main() {
	mux := http.NewServeMux()
	mux.HandleFunc("/ws-echo", func(res http.ResponseWriter, req *http.Request) {
		_, err := NewWebSocketConnection(res, req)
		if err != nil {
			log.Printf("error: %s", err.Error())
		}
	})
	server := http.Server{
		Addr:         ":12345",
		TLSConfig:    nil,
		ReadTimeout:  1 * time.Hour,
		WriteTimeout: 1 * time.Hour,
		Handler:      mux,
	}
	if err := server.ListenAndServe(); err != nil {
		log.Fatal(err)
	}
}
