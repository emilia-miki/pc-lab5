package mtype

import "fmt"

type MessageType uint8

const (
	SendData MessageType = iota
	StartCalculation
	GetStatus
	Error
)

func (mType MessageType) String() string {
	switch mType {
	case SendData:
		return "sendData"
	case StartCalculation:
		return "startCalculation"
	case GetStatus:
		return "getStatus"
	case Error:
		return "error"
	}

	return "unknown"
}

func (mType MessageType) Encode() (uint8, error) {
	if mType == Error {
		return 0, fmt.Errorf("Invalid request type: %s.", mType)
	}

	return uint8(mType), nil
}

func Decode(code uint8) (mType MessageType, err error) {
	if uint8(Error) < code {
		err = fmt.Errorf("Unknown message type code: %d.", mType)
		return
	}

	return MessageType(code), nil
}
