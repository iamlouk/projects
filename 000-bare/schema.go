package bare

import (
	"errors"
	"fmt"
	"strconv"
	"strings"
	"unicode"
)

type SLoc struct {
	File string
	Line int32
	Colm int32
}

type BareTypeKind uint

const (
	BareUInt BareTypeKind = iota
	BareU8
	BareU16
	BareU32
	BareU64
	BareInt
	BareI8
	BareI16
	BareI32
	BareI64
	BareF32
	BareF64
	BareBool
	BareString
	BareData
	BareDataFixed
	BareVoid
	BareEnum
	BareOptional
	BareList
	BareListFixed
	BareMap
	BareUnion
	BareStruct
	BareUserType
)

type BareType struct {
	Kind           BareTypeKind
	Name           string
	FixedSize      uint /* Optional, used for fixed-sized data/list variants. */
	ValueTy, KeyTy *BareType
	Fields         []Field /* Only for structs, unions, and unions. */
}

func (ty *BareType) Check() error {
	/* TODO: Check for legality: unique types in unions, map keys must be comparable/hashable, ... */
	return nil
}

func (ty *BareType) ToString(b *strings.Builder, asDecl bool) error {
	if asDecl {
		b.WriteString("type ")
		b.WriteString(ty.Name)
		b.WriteString(" ")
	} else if ty.Name != "" {
		b.WriteString(ty.Name)
		return nil
	}

	switch ty.Kind {
	case BareUInt:
		b.WriteString("uint")
	case BareU8:
		b.WriteString("u8")
	case BareU16:
		b.WriteString("u16")
	case BareU32:
		b.WriteString("u32")
	case BareU64:
		b.WriteString("u64")
	case BareInt:
		b.WriteString("int")
	case BareI8:
		b.WriteString("i8")
	case BareI16:
		b.WriteString("i16")
	case BareI32:
		b.WriteString("i32")
	case BareI64:
		b.WriteString("i64")
	case BareF32:
		b.WriteString("f32")
	case BareF64:
		b.WriteString("f64")
	case BareBool:
		b.WriteString("bool")
	case BareString:
		b.WriteString("str")
	case BareData:
		b.WriteString("data")
	case BareDataFixed:
		fmt.Fprintf(b, "data[%d]", ty.FixedSize)
	case BareVoid:
		b.WriteString("void")
	case BareEnum:
		b.WriteString("enum {")
		expectedEncoding := uint(0)
		for _, f := range ty.Fields {
			b.WriteString("\n  ")
			b.WriteString(f.Name)
			if f.Encoding != expectedEncoding {
				fmt.Fprintf(b, " = %d", f.Encoding)
				expectedEncoding = f.Encoding + 1
			} else {
				expectedEncoding += 1
			}
		}
		b.WriteString("\n}")
	case BareOptional:
		b.WriteString("optional<")
		ty.ValueTy.ToString(b, false)
		b.WriteString(">")
	case BareList:
		b.WriteString("list<")
		ty.ValueTy.ToString(b, false)
		b.WriteString(">")
	case BareListFixed:
		b.WriteString("list<")
		ty.ValueTy.ToString(b, false)
		fmt.Fprintf(b, ">[%d]", ty.FixedSize)
	case BareMap:
		b.WriteString("map<")
		ty.KeyTy.ToString(b, false)
		b.WriteString("><")
		ty.ValueTy.ToString(b, false)
		b.WriteString(">")
	case BareUnion:
		b.WriteString("union {")
		for i, f := range ty.Fields {
			if i == 0 {
				b.WriteString("\n  ")
			} else {
				b.WriteString(" |\n  ")
			}
			f.Type.ToString(b, false)
		}
		b.WriteString("\n}")
	case BareStruct:
		b.WriteString("struct {")
		for _, f := range ty.Fields {
			b.WriteString("\n  ")
			b.WriteString(f.Name)
			b.WriteString(": ")
			f.Type.ToString(b, false)
		}
		b.WriteString("\n}")
	default:
		panic("unimplemented")
	}
	b.WriteString("\n")
	return nil
}

type Field struct {
	Name     string
	Encoding uint
	Type     *BareType
}

type Parser struct {
	Filename  string
	input     string
	sloc      SLoc
	offset    int
	types     map[string]*BareType
	UserTypes []*BareType
}

func SchemaParser(filename string, input string) *Parser {
	p := &Parser{
		Filename: filename,
		input:    input,
		sloc: SLoc{
			File: filename,
			Line: 1,
			Colm: 1,
		},
		offset: 0,
		types: map[string]*BareType{
			"uint": {Kind: BareUInt, Name: "uint"},
			"u8":   {Kind: BareU8, Name: "u8"},
			"u16":  {Kind: BareU16, Name: "u16"},
			"u32":  {Kind: BareU32, Name: "u32"},
			"u64":  {Kind: BareU64, Name: "u64"},
			"int":  {Kind: BareInt, Name: "int"},
			"i8":   {Kind: BareI8, Name: "i8"},
			"i16":  {Kind: BareI16, Name: "i16"},
			"i32":  {Kind: BareI32, Name: "i32"},
			"i64":  {Kind: BareI64, Name: "i64"},
			"f32":  {Kind: BareF32, Name: "f32"},
			"f64":  {Kind: BareF64, Name: "f64"},
			"bool": {Kind: BareBool, Name: "bool"},
			"str":  {Kind: BareString, Name: "str"},
			"void": {Kind: BareVoid, Name: "void"},
		},
		UserTypes: make([]*BareType, 0),
	}
	return p
}

func (p *Parser) Parse() error {
	for {
		p.skipWhitespace()
		if p.offset >= len(p.input) {
			return nil
		}

		kw, err := p.expectIdentifier()
		if err != nil {
			return fmt.Errorf("expecting 'type': %w", err)
		}

		switch kw {
		case "type":
			name, err := p.expectIdentifier()
			if err != nil {
				return err
			}
			if !unicode.IsUpper(rune(name[0])) {
				return fmt.Errorf("sloc=%+v: type names must start with uppercase, got: %+v", p.sloc, name)
			}
			ty, err := p.Type()
			if err != nil {
				return err
			}

			t := *ty
			t.Name = name
			if err := t.Check(); err != nil {
				return err
			}
			p.types[name] = &t
			p.UserTypes = append(p.UserTypes, &t)
		default:
			return fmt.Errorf("sloc=%+v: expected 'type'", p.sloc)
		}
	}
}

func (p *Parser) ToString() (string, error) {
	b := strings.Builder{}

	for _, t := range p.UserTypes {
		if err := t.ToString(&b, true); err != nil {
			return "", err
		}
		b.WriteRune('\n')
	}

	return b.String(), nil
}

func (p *Parser) Type() (*BareType, error) {
	id, err := p.expectIdentifier()
	if err != nil {
		return nil, err
	}

	switch id {
	case "data":
		if p.peekNext() == '[' {
			p.expectRune('[')
			len, err := p.expectNumber()
			if err != nil {
				return nil, err
			}
			if err := p.expectRune(']'); err != nil {
				return nil, err
			}
			return &BareType{Kind: BareDataFixed, FixedSize: len}, nil
		} else {
			return &BareType{Kind: BareData}, nil
		}
	case "enum":
		if err := p.expectRune('{'); err != nil {
			return nil, err
		}
		t := &BareType{Kind: BareEnum, ValueTy: p.types["uint"]}
		maxEncoding := uint(0)
		for {
			if p.peekNext() == '}' {
				p.offset += 1
				break
			}

			name, err := p.expectAllUpperIdentifier()
			if err != nil {
				return nil, err
			}

			encoding := uint(0)
			if p.peekNext() == '=' {
				p.offset += 1
				encoding, err = p.expectNumber()
				if err != nil {
					return nil, err
				}
				maxEncoding = encoding
			} else {
				encoding = maxEncoding
				maxEncoding += 1
			}

			t.Fields = append(t.Fields, Field{
				Name:     name,
				Encoding: encoding,
			})
		}
		return t, nil
	case "optional":
		t, err := p.typeInAngleBrackets()
		if err != nil {
			return nil, err
		}
		return &BareType{Kind: BareOptional, ValueTy: t}, nil
	case "list":
		t, err := p.typeInAngleBrackets()
		if err != nil {
			return nil, err
		}
		if p.peekNext() == '[' {
			p.expectRune('[')
			len, err := p.expectNumber()
			if err != nil {
				return nil, err
			}
			if err := p.expectRune(']'); err != nil {
				return nil, err
			}
			return &BareType{Kind: BareListFixed, FixedSize: len, ValueTy: t}, nil
		} else {
			return &BareType{Kind: BareList, ValueTy: t}, nil
		}
	case "map":
		keyty, err := p.typeInAngleBrackets()
		if err != nil {
			return nil, err
		}
		valty, err := p.typeInAngleBrackets()
		if err != nil {
			return nil, err
		}
		return &BareType{Kind: BareMap, KeyTy: keyty, ValueTy: valty}, nil
	case "union":
		if err := p.expectRune('{'); err != nil {
			return nil, err
		}
		t := &BareType{Kind: BareUnion}
		if p.peekNext() == '}' {
			p.offset += 1
			return t, nil
		}
		for {
			ty, err := p.Type()
			if err != nil {
				return nil, err
			}
			if p.peekNext() == '=' {
				panic("todo")
			}
			t.Fields = append(t.Fields, Field{
				Type: ty,
			})

			r := p.peekNext()
			p.offset += 1
			if r == '}' {
				break
			}
			if r != '|' {
				return nil, errors.New("expected '|' or '}'")
			}
		}
		return t, nil
	case "struct":
		if err := p.expectRune('{'); err != nil {
			return nil, err
		}
		t := &BareType{Kind: BareStruct}
		for {
			if p.peekNext() == '}' {
				p.offset += 1
				break
			}

			name, err := p.expectIdentifier()
			if err != nil {
				return nil, err
			}
			if err := p.expectRune(':'); err != nil {
				return nil, err
			}
			fieldty, err := p.Type()
			if err != nil {
				return nil, err
			}
			t.Fields = append(t.Fields, Field{
				Name: name,
				Type: fieldty,
			})
		}
		return t, nil
	default:
		if t, ok := p.types[id]; ok {
			return t, nil
		}
	}

	return nil, fmt.Errorf("sloc=%+v: expected type, found invalid identifier", p.sloc)
}

func (p *Parser) typeInAngleBrackets() (*BareType, error) {
	if err := p.expectRune('<'); err != nil {
		return nil, err
	}
	ty, err := p.Type()
	if err != nil {
		return nil, err
	}
	if err := p.expectRune('>'); err != nil {
		return nil, err
	}
	return ty, nil
}

func (p *Parser) skipWhitespace() {
	for p.offset < len(p.input) {
		r := rune(p.input[p.offset])
		if r == '#' {
			for p.offset < len(p.input) {
				r = rune(p.input[p.offset])
				if r == '\n' {
					break
				}
				p.offset += 1
			}
		}

		if !unicode.IsSpace(r) {
			break
		}

		if rune(p.input[p.offset]) == '\n' {
			p.sloc.Line += 1
			p.sloc.Colm = 0
		}
		p.sloc.Colm += 1
		p.offset += 1
	}
}

func (p *Parser) expectIdentifier() (string, error) {
	p.skipWhitespace()
	i := 0
	for ; p.offset+i < len(p.input); i++ {
		c := rune(p.input[p.offset+i])
		if i == 0 && !unicode.IsLetter(c) {
			return "", fmt.Errorf("sloc=%+v, c=%+v: identifier expected", p.sloc, c)
		} else if !unicode.IsLetter(c) && !unicode.IsDigit(c) && c != '_' {
			break
		}
	}

	if i == 0 {
		return "", errors.New("unexpected EOF (expected identifier)")
	}

	id := p.input[p.offset : p.offset+i]
	p.offset += i
	return id, nil
}

func (p *Parser) expectAllUpperIdentifier() (string, error) {
	id, err := p.expectIdentifier()
	if err != nil {
		return id, err
	}

	for _, c := range id {
		if c != '_' && !unicode.IsUpper(c) {
			return id, fmt.Errorf("sloc=%+v, id=%+v: Expected only uppercase letters", p.sloc, id)
		}
	}

	return id, nil
}

func (p *Parser) expectRune(r rune) error {
	p.skipWhitespace()
	if p.offset >= len(p.input) {
		return errors.New("unexpected EOF")
	}

	c := rune(p.input[p.offset])
	if c != r {
		return fmt.Errorf("sloc=%#v, c='%c': expected '%c'", p.sloc, c, r)
	}

	p.offset += 1
	return nil
}

func (p *Parser) peekNext() rune {
	p.skipWhitespace()
	if p.offset < len(p.input) {
		return rune(p.input[p.offset])
	}
	return 0
}

func (p *Parser) expectNumber() (uint, error) {
	p.skipWhitespace()
	i := 0
	for ; p.offset+i < len(p.input); i++ {
		c := rune(p.input[p.offset+i])
		if i == 0 && !unicode.IsDigit(c) {
			return 0, fmt.Errorf("sloc=%+v, c=%+v: expected a number/digit", p.sloc, c)
		} else if !unicode.IsDigit(c) {
			break
		}
	}

	if i == 0 {
		return 0, errors.New("unexpected EOF")
	}

	res, err := strconv.ParseUint(p.input[p.offset:p.offset+i], 10, 64)
	if err != nil {
		return 0, fmt.Errorf("sloc=%+v: expected a number: %w", p.sloc, err)
	}
	p.offset += i
	return uint(res), nil
}
