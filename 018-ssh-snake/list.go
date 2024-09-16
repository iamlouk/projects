package main

type Pos struct { X, Y int }

type Section struct {
	Pos Pos
	Next, Prev *Section
}

type Snake struct {
	Direction  Pos
	Head, Tail *Section
	Length int
}

func (s *Snake) Init(x, y int, length int) {
	e := &Section{ Pos: Pos{ X: x, Y: y } }
	s.Head = e
	s.Length = length
	for i := 0; i < length; i++ {
		next := &Section{ Pos: Pos{ X: x, Y: y } }
		next.Prev = e
		e.Next = next
		e = next
		s.Tail = e
	}
}

func (s *Snake) Contains(pos Pos) bool {
	for e := s.Head; e != nil; e = e.Next {
		if e.Pos == pos {
			return true
		}
	}
	return false
}

func (s *Snake) Move(w, h int) (Pos, Pos) {
	oldpos := s.Tail.Pos
	e := s.Tail
	s.Tail = e.Prev
	s.Tail.Next = nil
	e.Prev = nil
	e.Next = s.Head

	pos := s.Head.Pos
	pos.X += s.Direction.X
	pos.Y += s.Direction.Y
	if pos.X  < 0 { pos.X = w - 1 }
	if pos.X >= w { pos.X = 0     }
	if pos.Y  < 0 { pos.Y = h - 1 }
	if pos.Y >= h { pos.Y = 0     }

	e.Pos = pos
	s.Head.Prev = e
	s.Head = e
	return pos, oldpos
}

func (s *Snake) Grow() {
	e := &Section{
		Pos: s.Tail.Pos,
		Prev: s.Tail,
		Next: nil,
	}
	s.Tail.Next = e
	s.Tail = e
}

func (s *Snake) ToArray() []Pos {
	arr := make([]Pos, 0, s.Length)
	for e := s.Head; e != nil; e = e.Next {
		arr = append(arr, e.Pos)
	}
	return arr
}

