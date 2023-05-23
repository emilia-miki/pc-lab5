package matrix

import (
	"client/constants"
	"client/matrix/mtype"
	"fmt"
	"net"
	"os"
	"strings"
)

const CHUNK_SIZE = 1500

type Matrix struct {
	Type      mtype.MatrixType
	Dimension uint32
	FilePath  string
}

func (matrix Matrix) GetByteLen() uint64 {
	return uint64(matrix.Type.GetByteSize()) * uint64(matrix.Dimension) * uint64(matrix.Dimension)
}

func (matrix Matrix) String() string {
	return fmt.Sprintf("%dx%d matrix of type %s", matrix.Dimension, matrix.Dimension, matrix.Type)
}

func (matrix Matrix) FromTCPStreamToFile(con net.Conn) error {
	clientId := uint16(con.LocalAddr().(*net.TCPAddr).Port)
	downloadsFolder := fmt.Sprintf("%s/%d", constants.DOWNLOADS_FOLDER, clientId)
	os.Mkdir(downloadsFolder, os.ModeDir|os.ModePerm)

	originalFilePathParts := strings.Split(matrix.FilePath, "/")
	originalFileName := originalFilePathParts[len(originalFilePathParts)-1]
	downloadedFileName := fmt.Sprintf("%s/%s", downloadsFolder, originalFileName)

	downloadedFile, err := os.Create(downloadedFileName)
	if err != nil {
		return fmt.Errorf("Error creating file %s: %s", downloadedFileName, err)
	}
	defer downloadedFile.Close()

	chunks := make(chan []uint8)
	defer close(chunks)

	matrixByteLen := matrix.GetByteLen()
	chunkCount := uint32((matrixByteLen-1)/CHUNK_SIZE + 1)
	go func() {
		leftToRead := matrixByteLen
		for i := uint32(0); i < chunkCount; i++ {
			var chunkLen int
			if leftToRead < CHUNK_SIZE {
				chunkLen = int(leftToRead)
			} else {
				chunkLen = CHUNK_SIZE
			}
			leftToRead -= uint64(chunkLen)

			chunk := make([]uint8, chunkLen)

			totalN := 0
			for totalN < chunkLen {
				n, err := con.Read(chunk[totalN:])

				if err != nil {
					fmt.Fprintf(os.Stderr, "Error reading chunk %d from TCP stream: %s\n", i, err)
				}

				if uint32(n) == 0 {
					fmt.Fprintln(os.Stderr, "The server disconnected")
					os.Exit(1)
				}

				totalN += n
			}

			chunks <- chunk
		}
	}()

	for i := uint32(0); i < chunkCount; i++ {
		_, err = downloadedFile.Write(<-chunks)
		if err != nil {
			return fmt.Errorf("Error writing to file %s: %s", downloadedFileName, err)
		}
	}

	return nil
}

func (matrix Matrix) FromFileToTCPStream(con net.Conn) error {
	file, err := os.Open(matrix.FilePath)
	if err != nil {
		return fmt.Errorf("Error opening file %s: %s", matrix.FilePath, err)
	}
	defer file.Close()

	chunks := make(chan []uint8)
	defer close(chunks)

	matrixByteLen := matrix.GetByteLen()
	chunkCount := uint32((matrixByteLen-1)/CHUNK_SIZE + 1)
	go func() {
		leftToWrite := matrixByteLen
		for i := uint32(0); i < chunkCount; i++ {
			var chunkLen uint32
			if leftToWrite < CHUNK_SIZE {
				chunkLen = uint32(leftToWrite)
			} else {
				chunkLen = CHUNK_SIZE
			}
			leftToWrite -= uint64(chunkLen)

			chunk := make([]uint8, chunkLen)

			_, err := file.Read(chunk)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error reading chunk %d from matrix file: %s\n", i, err)
				os.Exit(1)
			}

			chunks <- chunk
		}
	}()

	for i := uint32(0); i < chunkCount; i++ {
		n, err := con.Write(<-chunks)

		if err != nil {
			return fmt.Errorf("Error writing chunk %d to TCPStream: %s", i, err)
		}

		if n == 0 {
			return fmt.Errorf("The server disconnected")
		}
	}

	return nil
}
