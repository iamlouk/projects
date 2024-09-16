package bare

import (
	"bytes"
	"testing"
)

func TestDecodeIntoUInt(t *testing.T) {
	r := bytes.NewBuffer([]byte{0xff, 0x01})
	var res uint
	if err := Decode(r, &res); err != nil {
		t.Fatal(err)
	}

	if res != 255 {
		t.Fatal("expected 255")
	}
}

func TestDecodeIntoUIntArray(t *testing.T) {
	r := bytes.NewBuffer([]byte{0xfe, 0x03, 0x7e, 0x80, 0x01})
	var res [3]int
	if err := Decode(r, &res); err != nil {
		t.Fatal(err)
	}

	if res[0] != 255 || res[1] != 63 || res[2] != 64 {
		t.Fatalf("unexpected value: %#v", res)
	}
}

func TestDecodeUInt(t *testing.T) {
	tests := []struct {
		Encoded []byte
		Res     uint
	}{
		{
			Encoded: []byte{0x00},
			Res:     0,
		},
		{
			Encoded: []byte{0x01},
			Res:     1,
		},
		{
			Encoded: []byte{0x7e},
			Res:     126,
		},
		{
			Encoded: []byte{0x7f},
			Res:     127,
		},
		{
			Encoded: []byte{0x80, 0x01},
			Res:     128,
		},
		{
			Encoded: []byte{0x81, 0x01},
			Res:     129,
		},
		{
			Encoded: []byte{0xff, 0x01},
			Res:     255,
		},
	}

	for _, test := range tests {
		r := bytes.NewBuffer(test.Encoded)
		decoded, err := DecodeUInt(r)
		if err != nil {
			t.Fatal(err)
		}
		if decoded != test.Res {
			t.Fatalf("expected: %#v, got: %#v", test.Res, decoded)
		}
		if r.Available() != 0 {
			t.Fatal("leftover bytes")
		}
	}
}

func TestDecodeInt(t *testing.T) {
	tests := []struct {
		Encoded []byte
		Res     int
	}{
		{
			Encoded: []byte{0x00},
			Res:     0,
		},
		{
			Encoded: []byte{0x02},
			Res:     1,
		},
		{
			Encoded: []byte{0x01},
			Res:     -1,
		},
		{
			Encoded: []byte{0x7e},
			Res:     63,
		},
		{
			Encoded: []byte{0x7d},
			Res:     -63,
		},
		{
			Encoded: []byte{0x80, 0x01},
			Res:     64,
		},
		{
			Encoded: []byte{0x7f},
			Res:     -64,
		},
		{
			Encoded: []byte{0x82, 0x01},
			Res:     65,
		},
		{
			Encoded: []byte{0x81, 0x01},
			Res:     -65,
		},
		{
			Encoded: []byte{0xfe, 0x03},
			Res:     255,
		},
		{
			Encoded: []byte{0xfd, 0x03},
			Res:     -255,
		},
	}

	for _, test := range tests {
		r := bytes.NewBuffer(test.Encoded)
		decoded, err := DecodeInt(r)
		if err != nil {
			t.Fatal(err)
		}
		if decoded != test.Res {
			t.Fatalf("expected: %#v, got: %#v", test.Res, decoded)
		}
		if r.Available() != 0 {
			t.Fatal("leftover bytes")
		}
	}
}

func TestStruct(t *testing.T) {
	s := struct {
		Foo  uint
		Bar  int
		Buzz string
	}{}
	r := bytes.NewBuffer([]byte{0xFF, 0x01, 0xFD, 0x03, 0x04, 0x42, 0x41, 0x52, 0x45})

	if err := Decode(r, &s); err != nil {
		t.Fatal(err)
	}

	if s.Foo != 255 || s.Bar != -255 || s.Buzz != "BARE" {
		t.Fatalf("expected a different struct: %#v", s)
	}
}
