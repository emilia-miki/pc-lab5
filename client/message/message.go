package message

import (
	"client/matrix"
	matrixType "client/matrix/mtype"
	"client/message/kind"
	messageType "client/message/mtype"
	"client/status"
	"fmt"
	"net"
	"strconv"
	"time"
	"unsafe"
)

type Message struct {
	Client  uint16
	Time    time.Time
	Kind    kind.Kind
	Type    messageType.MessageType
	Payload map[string]string
}

type Request Message
type Response Message

func NewRequest(client uint16, mType messageType.MessageType, payload map[string]string) Request {
	return Request{
		Client:  client,
		Kind:    kind.Request,
		Type:    mType,
		Payload: payload,
	}
}

var matrices map[uint8]matrix.Matrix = make(map[uint8]matrix.Matrix)

func (request *Request) Execute(con net.Conn) (resp Response, err error) {
	buffer := [5]uint8{}
	buffer[0], err = request.Type.Encode()
	if err != nil {
		return
	}
	con.Write(buffer[:1])

	var parsed uint64
	switch request.Type {
	case messageType.Reserve:
		var mType matrixType.MatrixType
		mType, err = matrixType.FromString(request.Payload["matrixType"])
		if err != nil {
			err = fmt.Errorf("error processing a %s request: %s", request.Type, request.Payload["matrixType"])
			return
		}

		parsed, err = strconv.ParseUint(request.Payload["matrixDimension"], 10, 32)
		if err != nil {
			err = fmt.Errorf("error processing a %s request: error parsing matrixDimension: %s", request.Type, err)
			return
		}
		mDimension := uint32(parsed)

		buffer[0] = mType.Encode()
		*(*uint32)(unsafe.Pointer(&buffer[1])) = mDimension
		_, err = con.Write(buffer[:5])
		if err != nil {
			err = fmt.Errorf("error processing a %s request: error writing to TCP stream: %s", request.Type, err)
			return
		}
		break
	case messageType.Calc:
		parsed, err = strconv.ParseUint(request.Payload["index"], 10, 8)
		if err != nil {
			err = fmt.Errorf("error processing a %s request: error parsing index: %s", request.Type, err)
			return
		}
		index := uint8(parsed)

		parsed, err = strconv.ParseUint(request.Payload["threadCount"], 10, 8)
		if err != nil {
			err = fmt.Errorf("error processing a %s request: error parsing threadCount: %s", request.Type, err)
			return
		}
		threadCount := uint8(parsed)

		buffer[0] = index
		buffer[1] = threadCount
		_, err = con.Write(buffer[:2])
		if err != nil {
			err = fmt.Errorf("error processing a %s request: error writing to TCP stream: %s", request.Type, err)
			return
		}

		matrix, ok := matrices[index]
		if !ok {
			err = fmt.Errorf("error processing a %s request: You have to reserve "+
				"a matrix before requesting calculation", request.Type)
			return
		}
		err = matrix.FromFileToTCPStream(con)
		if err != nil {
			err = fmt.Errorf("error processing a %s request: error writing to TCP stream: %s", request.Type, err)
			return
		}
		break
	case messageType.Poll:
		parsed, err = strconv.ParseUint(request.Payload["index"], 10, 8)
		if err != nil {
			err = fmt.Errorf("error processing a %s request: error parsing index: %s", request.Type, err)
			return
		}
		index := uint8(parsed)

		buffer[0] = index
		_, err = con.Write(buffer[:1])
		if err != nil {
			err = fmt.Errorf("error processing a %s request: error writing to TCP stream: %s", request.Type, err)
			return
		}
		break
	default:
		err = fmt.Errorf("Invalid request type: %s", request.Type)
		return
	}
	request.Time = time.Now()

	responsePayload := map[string]string{}

	_, err = con.Read(buffer[:1])
	if err != nil {
		err = fmt.Errorf("error reading response type code from TCP stream: %s", err)
		return
	}
	responseCode := buffer[0]

	responseType, err := messageType.Decode(responseCode)
	if err != nil {
		return
	}

	switch responseType {
	case messageType.Reserve:
		_, err = con.Read(buffer[:1])
		if err != nil {
			err = fmt.Errorf("error processing a %s response: error writing to TCP stream: %s", responseType, err)
			return
		}

		index := buffer[0]
		responsePayload["index"] = strconv.FormatUint(uint64(index), 10)

		var mType matrixType.MatrixType
		mType, err = matrixType.FromString(request.Payload["matrixType"])
		if err != nil {
			err = fmt.Errorf("error processing a %s response: error parsing matrixType: %s", responseType, err)
			return
		}

		parsed, err = strconv.ParseUint(request.Payload["matrixDimension"], 10, 32)
		if err != nil {
			err = fmt.Errorf("error processing a %s response: error parsing matrixDimension: %s", responseType, err)
			return
		}
		mDimension := uint32(parsed)

		matrices[index] = matrix.Matrix{
			Type:      mType,
			Dimension: mDimension,
			FilePath:  fmt.Sprintf(request.Payload["file"]),
		}
		break
	case messageType.Poll:
		_, err = con.Read(buffer[:1])
		if err != nil {
			err = fmt.Errorf("error processing a %s response: error reading status type code from TCP stream: %s", responseType, err)
			return
		}

		var st status.Status
		st, err = status.Decode(buffer[0])
		if err != nil {
			return
		}
		responsePayload["status"] = st.String()

		if st == status.Completed {
			parsed, err = strconv.ParseUint(request.Payload["index"], 10, 8)
			if err != nil {
				err = fmt.Errorf("error processing a %s response: error parsing index: %s", responseType, err)
				return
			}
			index := uint8(parsed)
			matrix := matrices[index]

			err = matrix.FromTCPStreamToFile(con)
			if err != nil {
				err = fmt.Errorf("error processing a %s response: error downloading the %s from TCP stream: %s", responseType, matrix, err)
				return
			}

			responsePayload["matrixType"] = matrix.Type.String()
			responsePayload["matrixDimension"] = strconv.FormatUint(uint64(matrix.Dimension), 10)
			responsePayload["file"] = matrix.FilePath
		}
		break
	case messageType.Error:
		_, err = con.Read(buffer[:1])
		if err != nil {
			err = fmt.Errorf("error processing an %s response: error reading message length from TCP stream: %s", responseType, err)
			return
		}

		len := buffer[0]
		buffer := make([]uint8, len)
		_, err = con.Read(buffer)
		if err != nil {
			err = fmt.Errorf("error processing ar %s response: error reading message from TCP stream: %s", responseType, err)
			return
		}

		responsePayload["message"] = string(buffer)
		break
	}

	resp = Response{
		Client:  request.Client,
		Time:    time.Now(),
		Kind:    kind.Response,
		Type:    responseType,
		Payload: responsePayload,
	}
	return
}

func (message Message) JsonString() string {
	jsonPayload := ""
	for key, value := range message.Payload {
		jsonPayload += fmt.Sprintf(`,"%s":"%s"`, key, value)
	}

	return fmt.Sprintf(`{"client":"%d","time":"%d","kind":"%s","type":"%s"%s}`,
		message.Client, message.Time.UnixNano(), message.Kind, message.Type, jsonPayload)
}

func (request Request) JsonString() string {
	return Message(request).JsonString()
}

func (response Response) JsonString() string {
	return Message(response).JsonString()
}
