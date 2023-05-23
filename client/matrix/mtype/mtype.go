package mtype

import (
	"fmt"
	"strings"
)

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

func getSupportedTypes() []string {
	return []string{"u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "f32", "f64"}
}

func FromString(str string) (mType MatrixType, err error) {
	for i := uint8(0); i <= uint8(F64); i++ {
		mType = MatrixType(i)
		if str == mType.String() {
			return
		}
	}

	err = fmt.Errorf("Unknown matrix type %s. Known types are: %s",
		str, strings.Join(getSupportedTypes(), " "))
	return
}

func (mType MatrixType) GetByteSize() uint8 {
	switch mType {
	case U8:
		return 1
	case U16:
		return 2
	case U32:
		return 4
	case U64:
		return 8
	case I8:
		return 1
	case I16:
		return 2
	case I32:
		return 4
	case I64:
		return 8
	case F32:
		return 4
	case F64:
		return 8
	}

	panic("")
}

func (mType MatrixType) Encode() uint8 {
	return uint8(mType)
}

func Decode(code uint8) (mType MatrixType, err error) {
	if code > uint8(F64) {
		err = fmt.Errorf("Unknown type code %d", code)
		return
	}

	mType = MatrixType(code)
	return
}
