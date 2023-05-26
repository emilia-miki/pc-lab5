package mtype

import "fmt"

type MessageType uint8

const (
	Reserve MessageType = iota
	Calc
	Poll
	Error
)

func (mType MessageType) String() string {
	switch mType {
	case Reserve:
		return "reserve"
	case Calc:
		return "calc"
	case Poll:
		return "poll"
	case Error:
		return "error"
	}

	return "undefined"
}

func (mType MessageType) Encode() uint8 {
	return uint8(mType)
}

func Decode(code uint8) (mType MessageType, err error) {
	if code > uint8(Error) {
		err = fmt.Errorf("Unknown message type code: %d", code)
		return
	}

	mType = MessageType(code)
	return
}
