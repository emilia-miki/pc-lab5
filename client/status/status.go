package status

import "fmt"

type Status uint8

const (
	NoData Status = iota
	Ready
	Running
	Completed
)

func (status Status) String() string {
	switch status {
	case NoData:
		return "noData"
	case Ready:
		return "ready"
	case Running:
		return "running"
	case Completed:
		return "completed"
	}

	return "unknown"
}

func Decode(code uint8) (status Status, err error) {
	if code > uint8(Completed) {
		err = fmt.Errorf("Unknown status code: %d", code)
		return
	}

	status = Status(code)
	return
}
