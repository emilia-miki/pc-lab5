package mtype

import "fmt"

const SUPPORTED_TYPES string = "bool, u8, u16, u32, u64, i8, i16, i32, i64, f32, f64"

type MatrixType uint8

const (
	U8 MatrixType = iota
	U16
	U32
	U64
	I8
	I16
	I32
	I64
	F32
	F64
)

func (mType MatrixType) Encode() uint8 {
	return uint8(mType)
}

func FromString(str string) (mType MatrixType, err error) {
	switch str {
	case "u8":
		return U8, nil
	case "u16":
		return U16, nil
	case "u32":
		return U32, nil
	case "u64":
		return U64, nil
	case "i8":
		return I8, nil
	case "i16":
		return I16, nil
	case "i32":
		return I32, nil
	case "i64":
		return I64, nil
	case "f32":
		return F32, nil
	case "f64":
		return F64, nil
	}

	err = fmt.Errorf("Unknown matrix type %s. Known types are: %s.", str, SUPPORTED_TYPES)
	return
}

func (mType MatrixType) String() string {
	switch mType {
	case U8:
		return "u8"
	case U16:
		return "u16"
	case U32:
		return "u32"
	case U64:
		return "u64"
	case I8:
		return "i8"
	case I16:
		return "i16"
	case I32:
		return "i32"
	case I64:
		return "i64"
	case F32:
		return "f32"
	case F64:
		return "f64"
	}

	return "undefined"
}

func (mType MatrixType) GetByteSize() (size uint8, err error) {
	switch mType {
	case U8:
		return 1, nil
	case U16:
		return 2, nil
	case U32:
		return 4, nil
	case U64:
		return 8, nil
	case I8:
		return 1, nil
	case I16:
		return 2, nil
	case I32:
		return 4, nil
	case I64:
		return 8, nil
	case F32:
		return 4, nil
	case F64:
		return 8, nil
	}

	err = fmt.Errorf("The list of types in GetByteSize in " +
		"client/matrix/mtype/mtype.go is probably not exhaustive.")
	return
}

func Decode(code uint8) (mType MatrixType, err error) {
	if uint8(F64) < code {
		err = fmt.Errorf("Unknown type code %d.", code)
		return
	}

	return MatrixType(code), nil
}
