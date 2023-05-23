package command

import (
	"fmt"
	"strings"
)

type Command uint8

const (
	Reserve Command = iota
	Calc
	Poll
	Close
)

func (command Command) String() string {
	switch command {
	case Reserve:
		return "reserve"
	case Calc:
		return "calc"
	case Poll:
		return "poll"
	case Close:
		return "close"
	}

	return "undefined"
}

func getSupportedCommands() []string {
	return []string{"reserve", "calc", "poll", "close"}
}

func CommandFromString(str string) (cmd Command, err error) {
	for i := uint8(0); i <= uint8(Close); i++ {
		cmd = Command(i)
		if str == cmd.String() {
			return
		}
	}

	err = fmt.Errorf("Unknown command: %s. Known commands are: %s",
		str, strings.Join(getSupportedCommands(), " "))
	return
}
