package matrix

import (
	"bufio"
	"client/constants"
	"client/matrix/mtype"
	"fmt"
	"net"
	"os"
	"strconv"
	"strings"
	"unsafe"
)

type Matrix struct {
	Type      mtype.MatrixType
	Dimension uint32
	Bytes     []uint8
}

func FromFile(mType mtype.MatrixType, mFileName string) (matrix Matrix, err error) {
	mFileHandle, err := os.Open(mFileName)
	if err != nil {
		err = fmt.Errorf("Error opening file %s: %s.", mFileName, err)
		return
	}

	defer mFileHandle.Close()

	scanner := bufio.NewScanner(mFileHandle)
	scanner.Scan()

	firstString := scanner.Text()
	tokens := strings.FieldsFunc(firstString, func(r rune) bool { return r == ',' || r == '\n' })

	mTypeSize, err := mType.GetByteSize()
	if err != nil {
		return
	}

	mDimension := uint32(len(tokens))
	var sendDataRequestPreludeSize uint8 = 1 + 1 + 4
	matrix = Matrix{
		Type:      mType,
		Dimension: mDimension,
		Bytes:     make([]uint8, uint32(mTypeSize)*mDimension*mDimension+uint32(sendDataRequestPreludeSize)),
	}

	for i := uint32(0); i < mDimension; i++ {
		for j, token := range tokens {
			index := uint32(sendDataRequestPreludeSize) + (i*mDimension+uint32(j))*uint32(mTypeSize)
			mPointer := unsafe.Pointer(&matrix.Bytes[index])

			switch mType {
			case mtype.U8, mtype.U16, mtype.U32, mtype.U64:
				var res uint64
				res, err = strconv.ParseUint(token, 10, 8*int(mTypeSize))
				switch mType {
				case mtype.U8:
					*(*uint8)(mPointer) = uint8(res)
					break
				case mtype.U16:
					*(*uint16)(mPointer) = uint16(res)
					break
				case mtype.U32:
					*(*uint32)(mPointer) = uint32(res)
					break
				case mtype.U64:
					*(*uint64)(mPointer) = uint64(res)
					break
				}
				break
			case mtype.I8, mtype.I16, mtype.I32, mtype.I64:
				var res int64
				res, err = strconv.ParseInt(token, 10, 8*int(mTypeSize))
				switch mType {
				case mtype.I8:
					*(*int8)(mPointer) = int8(res)
					break
				case mtype.I16:
					*(*int16)(mPointer) = int16(res)
					break
				case mtype.I32:
					*(*int32)(mPointer) = int32(res)
					break
				case mtype.I64:
					*(*int64)(mPointer) = int64(res)
					break
				}
				break
			case mtype.F32, mtype.F64:
				var res float64
				res, err = strconv.ParseFloat(token, 8*int(mTypeSize))
				switch mType {
				case mtype.F32:
					*(*float32)(mPointer) = float32(res)
					break
				case mtype.F64:
					*(*float64)(mPointer) = float64(res)
					break
				}
				break
			}

			if err != nil {
				err = fmt.Errorf("Error parsing token %s into type %s: %s.", token, mType, err)
				return
			}
		}

		res := scanner.Scan()
		if !res {
			break
		}

		tokens = strings.FieldsFunc(scanner.Text(), func(r rune) bool { return r == ',' || r == '\n' })
	}

	return
}

func FromTCPStream(con net.Conn) (matrix Matrix, err error) {
	preambleBuffer := [5]uint8{}
	_, err = con.Read(preambleBuffer[:])
	if err != nil {
		return
	}

	mType, err := mtype.Decode(preambleBuffer[0])
	if err != nil {
		return
	}

	mTypeSize, err := mType.GetByteSize()
	if err != nil {
		return
	}

	var mDimension uint32
	for i := 0; i < 4; i++ {
		mDimension |= uint32(preambleBuffer[1+i] >> (i * 8))
	}

	matrix = Matrix{
		Type:      mType,
		Dimension: mDimension,
		Bytes:     make([]uint8, uint32(mTypeSize)*mDimension*mDimension),
	}

	read_count, err := con.Read(matrix.Bytes)
	if read_count != len(matrix.Bytes) {
		err = fmt.Errorf("Expected to read %d bytes, but got %d", len(matrix.Bytes), read_count)
		return
	}
	return
}

func (matrix Matrix) ToFile(fileName string) error {
	_, err := os.Stat(constants.DOWNLOADS_FOLDER)
	if err != nil {
		err = os.Mkdir(constants.DOWNLOADS_FOLDER, os.ModePerm)

		if err != nil {
			return err
		}
	}

	file, err := os.Create(fileName)
	if err != nil {
		return err
	}

	defer file.Close()

	mTypeSize, err := matrix.Type.GetByteSize()
	if err != nil {
		return err
	}

	for i := uint32(0); i < matrix.Dimension; i++ {
		for j := uint32(0); j < matrix.Dimension; j++ {
			baseIndex := (i*matrix.Dimension + j) * uint32(mTypeSize)
			mPointer := unsafe.Pointer(&matrix.Bytes[baseIndex])
			var str string

			switch matrix.Type {
			case mtype.U8, mtype.U16, mtype.U32, mtype.U64:
				var orig uint64
				switch matrix.Type {
				case mtype.U8:
					orig = uint64(*(*uint8)(mPointer))
					break
				case mtype.U16:
					orig = uint64(*(*uint16)(mPointer))
					break
				case mtype.U32:
					orig = uint64(*(*uint32)(mPointer))
					break
				case mtype.U64:
					orig = uint64(*(*uint64)(mPointer))
					break
				}
				str = strconv.FormatUint(orig, 10)
				break
			case mtype.I8, mtype.I16, mtype.I32, mtype.I64:
				var orig int64
				switch matrix.Type {
				case mtype.I8:
					orig = int64(*(*int8)(mPointer))
					break
				case mtype.U16:
					orig = int64(*(*int16)(mPointer))
					break
				case mtype.U32:
					orig = int64(*(*int32)(mPointer))
					break
				case mtype.U64:
					orig = int64(*(*int64)(mPointer))
					break
				}
				str = strconv.FormatInt(orig, 10)
				break
			case mtype.F32, mtype.F64:
				var orig float64
				switch matrix.Type {
				case mtype.F32:
					orig = float64(*(*float32)(mPointer))
					break
				case mtype.F64:
					orig = float64(*(*float64)(mPointer))
					break
				}
				str = strconv.FormatFloat(orig, 'f', -1, 8*int(mTypeSize))
				break
			}

			if j == matrix.Dimension-1 {
				str += "\n"
			} else {
				str += ","
			}

			file.WriteString(str)
		}
	}

	return nil
}
