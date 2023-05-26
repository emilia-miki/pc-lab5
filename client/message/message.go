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

var matrices map[uint64]matrix.Matrix = make(map[uint64]matrix.Matrix)

func (request *Request) Execute(con net.Conn) (resp Response, err error) {
	buffer := [1]uint8{request.Type.Encode()}
	_, err = con.Write(buffer[:])
	if err != nil {
		err = fmt.Errorf("error writing request type code to TCP stream: %s", err)
		return
	}

	var parsed uint64
	var id uint64
	var mType matrixType.MatrixType
	var mDimension uint32
	switch request.Type {
	case messageType.Reserve:
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
		mDimension = uint32(parsed)

		buffer := [5]uint8{}
		buffer[0] = mType.Encode()
		*(*uint32)(unsafe.Pointer(&buffer[1])) = mDimension
		_, err = con.Write(buffer[:])
		if err != nil {
			err = fmt.Errorf("error processing a %s request: error writing to TCP stream: %s", request.Type, err)
			return
		}
		break
	case messageType.Calc:
		id, err = strconv.ParseUint(request.Payload["id"], 10, 64)
		if err != nil {
			err = fmt.Errorf("error processing a %s request: error parsing index: %s", request.Type, err)
			return
		}

		matrix, ok := matrices[id]
		if !ok {
			err = fmt.Errorf("error processing a %s request: You have to reserve "+
				"a matrix on id %d before requesting calculation on it", request.Type, id)
			return
		}

		buffer := [8]uint8{}
		*(*uint64)(unsafe.Pointer(&buffer[0])) = id
		_, err = con.Write(buffer[:])
		if err != nil {
			err = fmt.Errorf("error processing a %s request: error writing to TCP stream: %s", request.Type, err)
			return
		}

		err = matrix.FromFileToTCPStream(con)
		if err != nil {
			err = fmt.Errorf("error processing a %s request: error writing to TCP stream: %s", request.Type, err)
			return
		}
		break
	case messageType.Poll:
		id, err = strconv.ParseUint(request.Payload["id"], 10, 64)
		if err != nil {
			err = fmt.Errorf("error processing a %s request: error parsing index: %s", request.Type, err)
			return
		}

		buffer := [8]uint8{}
		*(*uint64)(unsafe.Pointer(&buffer[0])) = id
		_, err = con.Write(buffer[:])
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

	_, err = con.Read(buffer[:])
	if err != nil {
		err = fmt.Errorf("error reading response type code from TCP stream: %s", err)
		return
	}
	responseCode := buffer[0]

	responseType, err := messageType.Decode(responseCode)
	if err != nil {
		return
	}
	responsePayload := map[string]string{}
	switch responseType {
	case messageType.Reserve:
		buffer := [8]uint8{}
		_, err = con.Read(buffer[:])
		if err != nil {
			err = fmt.Errorf("error processing a %s response: error reading from TCP stream: %s", responseType, err)
			return
		}
		id = *(*uint64)(unsafe.Pointer(&buffer[0]))
		responsePayload["id"] = strconv.FormatUint(id, 10)

		filePath := request.Payload["file"]
		delete(request.Payload, "file")
		matrices[id] = matrix.Matrix{
			Type:      mType,
			Dimension: mDimension,
			FilePath:  filePath,
		}
		break
	case messageType.Poll:
		_, err = con.Read(buffer[:])
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
			matrix := matrices[id]

			err = matrix.FromTCPStreamToFile(con)
			if err != nil {
				err = fmt.Errorf("error processing a %s response: error downloading the %s from TCP stream: %s", responseType, matrix, err)
				return
			}

			responsePayload["matrixType"] = matrix.Type.String()
			responsePayload["matrixDimension"] = strconv.FormatUint(uint64(matrix.Dimension), 10)
		}
		break
	case messageType.Error:
		_, err = con.Read(buffer[:])
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
