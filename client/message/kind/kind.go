package kind

type Kind uint8

const (
	Request Kind = iota
	Response
)

func (kind Kind) String() string {
	switch kind {
	case Request:
		return "request"
	case Response:
		return "response"
	}

	return "undefined"
}
