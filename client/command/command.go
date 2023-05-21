package command

import "fmt"

const SUPPORTED_COMMANDS string = "sendData, startCalculation, getStatus, closeConnection"

type Command uint8

const (
	SendData Command = iota
	StartCalculation
	GetStatus
	CloseConnection
)

func (command Command) String() string {
	switch command {
	case SendData:
		return "sendData"
	case StartCalculation:
		return "startCalculation"
	case GetStatus:
		return "getStatus"
	case CloseConnection:
		return "closeConnection"
	}

	return "undefined"
}

func CommandFromString(str string) (cmd Command, err error) {
	switch str {
	case "sendData":
		return SendData, nil
	case "startCalculation":
		return StartCalculation, nil
	case "getStatus":
		return GetStatus, nil
	case "closeConnection":
		return CloseConnection, nil
	}

	err = fmt.Errorf("Unknown command: %s. Known commands are: %s.", str, SUPPORTED_COMMANDS)
	return
}
