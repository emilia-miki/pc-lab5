package message

import (
	"client/constants"
	"client/matrix"
	matrixType "client/matrix/mtype"
	"client/message/kind"
	messageType "client/message/mtype"
	"client/status"
	"encoding/json"
	"fmt"
	"net"
	"strconv"
	"strings"
	"time"
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

func (request *Request) Send(con net.Conn) (err error) {
	switch request.Type {
	case messageType.SendData:
		mType, err := matrixType.FromString(request.Payload["type"])
		if err != nil {
			return err
		}

		matrix, err := matrix.FromFile(mType, request.Payload["file"])
		if err != nil {
			return err
		}

		// the first 6 bytes are emtpy, the rest is filled with the matrix data
		buffer := matrix.Bytes

		buffer[0], err = request.Type.Encode()
		if err != nil {
			return err
		}

		buffer[1] = matrix.Type.Encode()

		for i := 0; i < 4; i++ {
			buffer[2+i] |= uint8(matrix.Dimension >> (i * 8))
		}

		_, err = con.Write(buffer)
		if err != nil {
			return err
		}
		break
	case messageType.StartCalculation:
		buffer := make([]uint8, 3)
		buffer[0], err = request.Type.Encode()
		parsed, err := strconv.ParseUint(request.Payload["index"], 10, 8)
		if err != nil {
			return err
		}

		buffer[1] = uint8(parsed)

		parsed, err = strconv.ParseUint(request.Payload["threadCount"], 10, 8)
		if err != nil {
			return err
		}

		buffer[2] = uint8(parsed)

		_, err = con.Write(buffer)
		if err != nil {
			return err
		}
		break
	case messageType.GetStatus:
		buffer := make([]uint8, 2)
		buffer[0], err = request.Type.Encode()
		if err != nil {
			return err
		}

		parsed, err := strconv.ParseUint(request.Payload["index"], 10, 8)
		if err != nil {
			return err
		}

		buffer[1] = uint8(parsed)

		_, err = con.Write(buffer)
		if err != nil {
			return err
		}
		break
	default:
		return fmt.Errorf("Invalid message type for request: %s.", request.Type)
	}

	request.Time = time.Now()
	return nil
}

func Receive(id uint16, con net.Conn) (response Response, err error) {
	var unitBuffer = [1]uint8{}

	_, err = con.Read(unitBuffer[:])
	if err != nil {
		return
	}

	var timeStamp time.Time
	mType, err := messageType.Decode(unitBuffer[0])
	if err != nil {
		return
	}

	payload := map[string]string{}

	switch mType {
	case messageType.SendData:
		_, err = con.Read(unitBuffer[:])
		if err != nil {
			return
		}

		payload["index"] = strconv.FormatUint(uint64(unitBuffer[0]), 10)
		break
	case messageType.GetStatus:
		_, err = con.Read(unitBuffer[:])
		if err != nil {
			return
		}

		var st status.Status
		st, err = status.Decode(unitBuffer[0])
		if err != nil {
			return
		}

		payload["status"] = st.String()
		if st == status.Completed {
			var m matrix.Matrix
			m, err = matrix.FromTCPStream(con)
			if err != nil {
				return
			}

			timeStamp = time.Now()

			fileName := fmt.Sprintf("%s/%s_%d_%d_matrix.csv",
				constants.DOWNLOADS_FOLDER, m.Type, m.Dimension, time.Now().UnixNano())
			err = m.ToFile(fileName)
			if err != nil {
				return
			}

			payload["type"] = m.Type.String()
			payload["dimension"] = strconv.FormatUint(uint64(m.Dimension), 10)
			payload["file"] = fileName
		}
		break
	case messageType.Error:
		_, err = con.Read(unitBuffer[:])
		if err != nil {
			return
		}

		len := unitBuffer[0]
		buffer := make([]uint8, len)
		_, err = con.Read(buffer)
		if err != nil {
			return
		}

		payload["message"] = string(buffer)
		break
	}

	var defaultTimeStamp time.Time
	if timeStamp == defaultTimeStamp {
		timeStamp = time.Now()
	}

	return Response{
		Client:  id,
		Time:    timeStamp,
		Kind:    kind.Response,
		Type:    mType,
		Payload: payload,
	}, nil
}

func (message Message) JsonString() string {
	jsonPayloadBytes, _ := json.Marshal(message.Payload)
	jsonPayload := string(jsonPayloadBytes)

	patterns := []string{`"index":`, `"dimension":`, `"threadCount":`}
	for _, pattern := range patterns {
		startIndex := strings.Index(jsonPayload, pattern)
		if startIndex == -1 {
			continue
		}
		startIndex += len(pattern)
		endIndex := strings.Index(jsonPayload[startIndex:], ",")
		if endIndex == -1 {
			endIndex = strings.Index(jsonPayload[startIndex:], "}")
			if endIndex == -1 {
				continue
			}
		}
		endIndex += startIndex
		enclosedString := jsonPayload[startIndex:endIndex]
		jsonPayload = fmt.Sprintf("%s%s%s", jsonPayload[:startIndex],
			enclosedString[1:len(enclosedString)-1], jsonPayload[endIndex:])
	}

	return fmt.Sprintf(`{"client":%d,"time":%d,"kind":"%s","type":"%s","payload":%s}`,
		message.Client, message.Time.UnixNano(), message.Kind, message.Type, jsonPayload)
}

func (request Request) JsonString() string {
	return Message(request).JsonString()
}

func (response Response) JsonString() string {
	return Message(response).JsonString()
}
