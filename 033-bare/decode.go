package bare

import (
	"errors"
	"fmt"
	"io"
	"reflect"
)

func DecodeUInt(r io.ByteReader) (uint, error) {
	res, shift := uint64(0), uint64(0)
	done := false
	for i := 0; !done; i++ {
		b, err := r.ReadByte()
		if err != nil {
			return 0, err
		}

		done = (b & 0b1000_0000) == 0
		if i == 9 && (!done || (b&0b0111_1110) != 0) {
			return 0, errors.New("the max. precision of a BARE uint is 64 bits")
		}

		res = res | ((uint64(b) & 0b0111_1111) << shift)
		shift += 7
	}
	return uint(res), nil
}

func DecodeInt(r io.ByteReader) (int, error) {
	ures, err := DecodeUInt(r)
	if err != nil {
		return 0, err
	}

	isneg := (ures & 0b1) == 1
	if isneg {
		return -(int(ures) >> 1) - 1, nil
	} else {
		return int(ures >> 1), nil
	}
}

func decodeValue(r io.ByteReader, dst reflect.Value) error {
	switch dst.Kind() {
	case reflect.Uint:
		res, err := DecodeUInt(r)
		if err != nil {
			return err
		}
		dst.SetUint(uint64(res))
		return nil
	case reflect.Int:
		res, err := DecodeInt(r)
		if err != nil {
			return err
		}
		dst.SetInt(int64(res))
		return nil
	case reflect.Array:
		n := dst.Type().Len()
		for i := range n {
			if err := decodeValue(r, dst.Index(i)); err != nil {
				return err
			}
		}
		return nil
	case reflect.Bool:
		b, err := r.ReadByte()
		if err != nil {
			return err
		}
		if b != 0 && b != 1 {
			return errors.New("expected zero or one for booleans")
		}

		dst.SetBool(b == 1)
		return nil
	case reflect.Slice:
		n, err := DecodeUInt(r)
		if err != nil {
			return fmt.Errorf("expected a error: %w", err)
		}
		sliceVal := reflect.MakeSlice(dst.Type().Elem(), int(n), int(n))
		for i := range int(n) {
			if err := decodeValue(r, dst.Index(i)); err != nil {
				return err
			}
		}

		dst.Set(sliceVal)
		return nil
	case reflect.Struct:
		fields := dst.NumField()
		for i := range fields {
			field := dst.Field(i)
			tag := dst.Type().Field(i).Tag
			skip := tag.Get("bare") == "-skip-"
			if skip {
				continue
			}
			if err := decodeValue(r, field); err != nil {
				return err
			}
		}
		return nil
	case reflect.String:
		n, err := DecodeUInt(r)
		if err != nil {
			return fmt.Errorf("expected a error: %w", err)
		}
		bytes := make([]byte, 0, n)
		for range n {
			b, err := r.ReadByte()
			if err != nil {
				return err
			}
			bytes = append(bytes, b)
		}
		dst.SetString(string(bytes))
		return nil
	default:
		return fmt.Errorf("unsupported type for BARE: %s", dst.Type().Name())
	}
}

func Decode(r io.ByteReader, dst interface{}) error {
	ptrToVal := reflect.ValueOf(dst)
	if ptrToVal.Type().Kind() != reflect.Pointer {
		return errors.New("expected a pointer to something")
	}

	return decodeValue(r, ptrToVal.Elem())
}
