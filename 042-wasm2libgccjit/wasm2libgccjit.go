package main

import (
	"bufio"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"log"
	"os"
	"reflect"
	"strings"
)

/*
#cgo LDFLAGS: -lgccjit
#include <libgccjit.h>
*/
import "C"

const (
	WASM_SECTION_CUSTOM     = 0
	WASM_SECTION_TYPE       = 1
	WASM_SECTION_IMPORT     = 2
	WASM_SECTION_FUNCTION   = 3
	WASM_SECTION_TABLE      = 4
	WASM_SECTION_MEMORY     = 5
	WASM_SECTION_GLOBAL     = 6
	WASM_SECTION_EXPORT     = 7
	WASM_SECTION_START      = 8
	WASM_SECTION_ELEMENT    = 9
	WASM_SECTION_CODE       = 10
	WASM_SECTION_DATA       = 11
	WASM_SECTION_DATA_COUNT = 12

	WASM_SECTION_MAX = 12
)

var SECTION_NAMES = map[int]string{
	WASM_SECTION_CUSTOM:     "custom",
	WASM_SECTION_TYPE:       "type",
	WASM_SECTION_IMPORT:     "import",
	WASM_SECTION_FUNCTION:   "function",
	WASM_SECTION_TABLE:      "table",
	WASM_SECTION_MEMORY:     "memory",
	WASM_SECTION_GLOBAL:     "global",
	WASM_SECTION_EXPORT:     "export",
	WASM_SECTION_START:      "start",
	WASM_SECTION_ELEMENT:    "element",
	WASM_SECTION_CODE:       "code",
	WASM_SECTION_DATA:       "data",
	WASM_SECTION_DATA_COUNT: "count",
}

const (
	INSTR_UNREACHABLE uint8 = 0x00
	INSTR_NOP         uint8 = 0x01

	INSTR_LOCAL_GET  uint8 = 0x20
	INSTR_LOCAL_SET  uint8 = 0x21
	INSTR_LOCAL_TEE  uint8 = 0x22
	INSTR_GLOBAL_GET uint8 = 0x23
	INSTR_GLOBAL_SET uint8 = 0x24

	INSTR_I32_CONST uint8 = 0x41
	INSTR_I64_CONST uint8 = 0x42
	INSTR_F32_CONST uint8 = 0x43
	INSTR_F64_CONST uint8 = 0x44

	INSTR_END uint8 = 0x0b
)

var INSTR_NAMES = map[uint8]string{
	INSTR_UNREACHABLE: "unreachable",
	INSTR_NOP:         "nop",
	INSTR_END:         "end",
	INSTR_LOCAL_GET:   "local.get",
	INSTR_LOCAL_SET:   "local.set",
	INSTR_LOCAL_TEE:   "local.tee",
	INSTR_GLOBAL_GET:  "global.get",
	INSTR_GLOBAL_SET:  "global.set",
	INSTR_I32_CONST:   "i32.const",
	INSTR_I64_CONST:   "i64.const",
	INSTR_F32_CONST:   "f32.const",
	INSTR_F64_CONST:   "f64.const",
}

type Instr struct {
	Opcode uint8
	Const  any
	Idx    int
}

func (i *Instr) String() string {
	switch i.Opcode {
	case INSTR_I32_CONST, INSTR_I64_CONST, INSTR_F32_CONST, INSTR_F64_CONST:
		return fmt.Sprintf("(%s %#v)", INSTR_NAMES[i.Opcode], i.Const)
	case INSTR_LOCAL_GET, INSTR_LOCAL_SET, INSTR_LOCAL_TEE, INSTR_GLOBAL_GET, INSTR_GLOBAL_SET:
		return fmt.Sprintf("(%s %d)", INSTR_NAMES[i.Opcode], i.Idx)
	default:
		return fmt.Sprintf("(%s)", INSTR_NAMES[i.Opcode])
	}
}

type Function struct {
	Idx    int
	Size   uint
	Type   reflect.Type
	Locals []struct {
		N    uint
		Type reflect.Type
	}
	Body []Instr
}

func ExprString(expr []Instr) string {
	b := strings.Builder{}
	for i, e := range expr {
		if i != 0 {
			b.WriteString(", ")
		}
		b.WriteString(e.String())
	}
	return b.String()
}

type Module struct {
	Version            uint32
	FunctionTypes      []reflect.Type
	FunctionTypeIndexs []int
	MemoryRanges       [][2]uint
	CustomSection      []struct {
		Size uint
		Name string
		Data []byte
	}
	Globals []struct {
		Type    reflect.Type
		Mutable bool
		Expr    []Instr
	}
	Exports []struct {
		Name string
		Kind string
		Idx  int
	}
	Functions []*Function
}

func ReadUnsignedLEB128(r *bufio.Reader) (uint, int, error) {
	num := uint(0)
	shift := uint(0)
	bytes := 0
	for {
		bytes += 1
		b, err := r.ReadByte()
		if err != nil {
			return 0, bytes, err
		}
		num |= (uint(b) & 0x7f) << shift
		if (b & 0x80) == 0 {
			return num, bytes, nil
		}
		shift += 7
	}
}

func ReadSignedLEB128(r *bufio.Reader, bits int) (int, int, error) {
	num := uint(0)
	shift := uint(0)
	bytes := 0
	for {
		bytes += 1
		b, err := r.ReadByte()
		if err != nil {
			return 0, bytes, err
		}
		num |= (uint(b) & 0x7f) << shift
		if (b & 0x80) == 0 {
			if shift < uint(bits) && (b&0x40) != 0 {
				num |= (^uint(0) << shift)
			}

			return int(num), bytes, nil
		}
		shift += 7
	}
}

func ParseType(r *bufio.Reader) (reflect.Type, error) {
	b, err := r.ReadByte()
	if err != nil {
		return nil, err
	}

	ParseResultType := func(r *bufio.Reader) ([]reflect.Type, error) {
		len, _, err := ReadUnsignedLEB128(r)
		if err != nil {
			return nil, err
		}

		res := make([]reflect.Type, 0, int(len))
		for i := uint(0); i < len; i++ {
			t, err := ParseType(r)
			if err != nil {
				return nil, err
			}
			res = append(res, t)
		}
		return res, nil
	}

	switch b {
	case 0x60:
		a, err := ParseResultType(r)
		if err != nil {
			return nil, err
		}
		b, err := ParseResultType(r)
		if err != nil {
			return nil, err
		}

		return reflect.FuncOf(a, b, false), nil
	case 0x7F:
		return reflect.TypeFor[int32](), nil
	case 0x7E:
		return reflect.TypeFor[int64](), nil
	case 0x7D:
		return reflect.TypeFor[float32](), nil
	case 0x7C:
		return reflect.TypeFor[float64](), nil
	default:
		return nil, fmt.Errorf("unkown type id: %d", b)
	}
}

func (m *Module) Parse(r *bufio.Reader) error {
	buf := make([]byte, 128)
	if _, err := io.ReadFull(r, buf[0:8]); err != nil {
		return err
	}

	if buf[0] != 0 || buf[1] != 'a' || buf[2] != 's' || buf[3] != 'm' {
		return fmt.Errorf("invalid magic number")
	}

	m.Version = binary.LittleEndian.Uint32(buf[4:8])
	if m.Version != 1 {
		return fmt.Errorf("unsupported version: %d", m.Version)
	}

	for {
		if err := m.ParseSection(r); err != nil {
			if err == io.EOF {
				break
			}
			return err
		}
	}

	log.Printf("done!")
	return nil
}

func (m *Module) ParseSection(r *bufio.Reader) error {
	id, err := r.ReadByte()
	if err != nil {
		return err
	}
	if id > WASM_SECTION_MAX {
		return fmt.Errorf("unknown section ID: %d", id)
	}

	size, _, err := ReadUnsignedLEB128(r)
	if err != nil {
		return err
	}

	buf := make([]byte, size)
	switch id {
	case WASM_SECTION_CUSTOM:
		namelen, n, err := ReadUnsignedLEB128(r)
		if err != nil {
			return err
		}
		datalen := size - (uint(n) + namelen)
		if _, err := io.ReadFull(r, buf[:namelen]); err != nil {
			return err
		}
		name := string(buf[:namelen])
		if _, err := io.ReadFull(r, buf[:datalen]); err != nil {
			return err
		}
		m.CustomSection = append(m.CustomSection, struct {
			Size uint
			Name string
			Data []byte
		}{
			Size: size,
			Name: name,
			Data: buf[:datalen],
		})
		log.Printf("section: id=%d (%s), size=%d, name=%#v", id, SECTION_NAMES[int(id)], size, name)
	case WASM_SECTION_TYPE:
		entries, _, err := ReadUnsignedLEB128(r)
		if err != nil {
			return err
		}
		log.Printf("section: id=%d (%s), size=%d, entries=%d", id, SECTION_NAMES[int(id)], size, entries)
		for i := 0; i < int(entries); i++ {
			t, err := ParseType(r)
			if err != nil {
				return err
			}
      if t.NumOut() > 1 {
        return fmt.Errorf("functions with more than one return value are currently unsupported", t.String())
      }
			m.FunctionTypes = append(m.FunctionTypes, t)
			log.Printf("-> type#%d: %s", i, t.String())
		}
	case WASM_SECTION_FUNCTION:
		entries, _, err := ReadUnsignedLEB128(r)
		if err != nil {
			return err
		}
		log.Printf("section: id=%d (%s), size=%d, entries=%d", id, SECTION_NAMES[int(id)], size, entries)
		for i := 0; i < int(entries); i++ {
			idx, _, err := ReadUnsignedLEB128(r)
			if err != nil {
				return err
			}
			m.FunctionTypeIndexs = append(m.FunctionTypeIndexs, int(idx))
			log.Printf("-> function#%d -> type#%d (%s)", i, idx, m.FunctionTypes[idx].String())
		}
	case WASM_SECTION_MEMORY:
		entries, _, err := ReadUnsignedLEB128(r)
		if err != nil {
			return err
		}
		for i := 0; i < int(entries); i++ {
			b, err := r.ReadByte()
			if err != nil {
				return err
			}
			switch b {
			case 0x00:
				min, _, err := ReadUnsignedLEB128(r)
				if err != nil {
					return err
				}
				m.MemoryRanges = append(m.MemoryRanges, [2]uint{min, ^uint(0)})
			case 0x01:
				min, _, err := ReadUnsignedLEB128(r)
				if err != nil {
					return err
				}
				max, _, err := ReadUnsignedLEB128(r)
				if err != nil {
					return err
				}
				m.MemoryRanges = append(m.MemoryRanges, [2]uint{min, max})
			default:
				return fmt.Errorf("expected 0x00 or 0x01 before limit", b)
			}
		}
		log.Printf("section: id=%d (%s), size=%d, ranges=%v", id, SECTION_NAMES[int(id)], size, m.MemoryRanges)
	case WASM_SECTION_GLOBAL:
		entries, _, err := ReadUnsignedLEB128(r)
		if err != nil {
			return err
		}
		log.Printf("section: id=%d (%s), size=%d, entried=%d", id, SECTION_NAMES[int(id)], size, entries)
		for i := 0; i < int(entries); i++ {
			t, err := ParseType(r)
			if err != nil {
				return err
			}
			ismut, err := r.ReadByte()
			if err != nil {
				return err
			}
			expr, err := m.ParseExpr(r)
			if err != nil {
				return err
			}
			m.Globals = append(m.Globals, struct {
				Type    reflect.Type
				Mutable bool
				Expr    []Instr
			}{
				Type: t, Mutable: ismut != 0, Expr: expr,
			})
			log.Printf("\tglobal: type=%s, mut=%v, expr=[%s]", t.String(), ismut != 0, ExprString(expr))
		}
	case WASM_SECTION_EXPORT:
		entries, _, err := ReadUnsignedLEB128(r)
		if err != nil {
			return err
		}
		log.Printf("section: id=%d (%s), size=%d, entried=%d", id, SECTION_NAMES[int(id)], size, entries)
		for i := 0; i < int(entries); i++ {
			namelen, _, err := ReadUnsignedLEB128(r)
			if err != nil {
				return err
			}
			if _, err := io.ReadFull(r, buf[:namelen]); err != nil {
				return err
			}
			name, kind := string(buf[:namelen]), ""
			b, err := r.ReadByte()
			if err != nil {
				return err
			}
			switch b {
			case 0x00:
				kind = "func"
			case 0x01:
				kind = "table"
			case 0x02:
				kind = "mem"
			case 0x03:
				kind = "global"
			default:
				return fmt.Errorf("unkown export desc. kind: %#v", b)
			}
			idx, _, err := ReadUnsignedLEB128(r)
			if err != nil {
				return err
			}
			log.Printf("\texport: name=%#v, kind=%s, idx=%d", name, kind, idx)
			m.Exports = append(m.Exports, struct {
				Name string
				Kind string
				Idx  int
			}{
				Name: name, Kind: kind, Idx: int(idx),
			})
		}
	case WASM_SECTION_CODE:
		entries, _, err := ReadUnsignedLEB128(r)
		if err != nil {
			return err
		}
		log.Printf("section: id=%d (%s), size=%d, entried=%d", id, SECTION_NAMES[int(id)], size, entries)
		for i := 0; i < int(entries); i++ {
			funcsize, _, err := ReadUnsignedLEB128(r)
			if err != nil {
				return err
			}
			f, err := m.ParseFunction(funcsize, r)
			if err != nil {
				return err
			}
			m.Functions = append(m.Functions, f)
		}
	default:
		log.Printf("section: id=%d (%s), size=%d -> Skipped!", id, SECTION_NAMES[int(id)], size)
		if _, err = r.Discard(int(size)); err != nil {
			return err
		}
	}

	return nil
}

func (m *Module) ParseInstr(r *bufio.Reader) (Instr, error) {
	b, err := r.ReadByte()
	if err != nil {
		return Instr{}, err
	}

	switch b {
	case INSTR_UNREACHABLE, INSTR_NOP, INSTR_END:
		return Instr{Opcode: uint8(b)}, nil
	case INSTR_LOCAL_GET, INSTR_LOCAL_SET, INSTR_LOCAL_TEE, INSTR_GLOBAL_GET, INSTR_GLOBAL_SET:
		idx, _, err := ReadUnsignedLEB128(r)
		if err != nil {
			return Instr{}, err
		}
		return Instr{Opcode: uint8(b), Idx: int(idx)}, nil
	case INSTR_I32_CONST:
		c, _, err := ReadSignedLEB128(r, 32)
		if err != nil {
			return Instr{}, err
		}
		return Instr{Opcode: uint8(b), Const: int32(c)}, nil
	default:
		return Instr{}, fmt.Errorf("unkown instruction opcode: %#v", b)
	}
}

func (m *Module) ParseExpr(r *bufio.Reader) ([]Instr, error) {
	instrs := make([]Instr, 0)
	for {
		instr, err := m.ParseInstr(r)
		if err != nil {
			return nil, err
		}
		instrs = append(instrs, instr)
		if instr.Opcode == INSTR_END {
			break
		}
	}
	return instrs, nil
}

func (m *Module) ParseFunction(size uint, r *bufio.Reader) (*Function, error) {
	numlocals, _, err := ReadUnsignedLEB128(r)
	if err != nil {
		return nil, err
	}
	f := &Function{
		Idx:  len(m.Functions),
		Size: size,
		Type: m.FunctionTypes[len(m.Functions)],
		Locals: make([]struct {
			N    uint
			Type reflect.Type
		}, 0, numlocals),
		Body: make([]Instr, 0),
	}

	for i := 0; i < int(numlocals); i++ {
		n, _, err := ReadUnsignedLEB128(r)
		if err != nil {
			return nil, err
		}
		t, err := ParseType(r)
		if err != nil {
			return nil, err
		}
		f.Locals = append(f.Locals, struct {
			N    uint
			Type reflect.Type
		}{
			N: n, Type: t,
		})
	}

	f.Body, err = m.ParseExpr(r)
	if err != nil {
		return nil, err
	}

	log.Printf("\tfunc: type=%s, idx=%d, #locals=%d, body=[%s]", f.Type.String(), f.Idx, len(f.Locals), ExprString(f.Body))
	return f, nil
}

func (m *Module) CodeGen(f *Function, ctx *C.gcc_jit_context) error {
  log.Printf("codegen: type=%s, idx=%d, #locals=%d, body=[%s]", f.Type.String(), f.Idx, len(f.Locals), ExprString(f.Body))

  getlibgcctype := func(t reflect.Type) (*C.gcc_jit_type, error) {
    res := (*C.gcc_jit_type)(nil)
    if t.Kind() == reflect.Int32 {
      res = C.gcc_jit_context_get_type(ctx, C.GCC_JIT_TYPE_INT32_T)
    } else if t.Kind() == reflect.Func {
      retty := C.gcc_jit_context_get_type(ctx, C.GCC_JIT_TYPE_VOID)
      if t.NumOut() > 0 {
        retty = getlibgcctype(t.Out(0))
      }

    } else {
      return nil, fmt.Errorf("unimplemented: %#v", t.String())
    }
    if res != nil {
      return nil, errors.New("failed to create type")
    }
    return res, nil
  }

  _ = getlibgcctype

	return nil
}

func main() {
	r := bufio.NewReader(os.Stdin)
	m := Module{}
	if err := m.Parse(r); err != nil {
		log.Fatal(err)
	}

  ctx := C.gcc_jit_context_acquire()
  if ctx == nil {
    log.Fatalf("failed to create libgccjit context")
  }
  defer C.gcc_jit_context_release(ctx)

  C.gcc_jit_context_set_bool_option(ctx, C.GCC_JIT_BOOL_OPTION_DUMP_GENERATED_CODE, 0);

  for _, f := range m.Functions {
    if err := m.CodeGen(f, ctx); err != nil {
      log.Fatal(err)
    }
  }


  C.gcc_jit_context_compile_to_file(ctx, C.GCC_JIT_OUTPUT_KIND_ASSEMBLER, C.CString("/dev/stdout"))
}
