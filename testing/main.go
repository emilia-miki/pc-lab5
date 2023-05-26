package main

import (
	"bufio"
	"bytes"
	"container/list"
	"context"
	crand "crypto/rand"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"io/ioutil"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"time"
)

const SERVER_FOLDER = "../server/target/release"
const SERVER_EXEC = "./server"

const CLIENT_FOLDER = "../client"
const CLIENT_EXEC = "./client"

const TEST_MATRICES_FOLDER = "test_matrices"
const DOWNLOADS_FOLDER_NAME = "downloaded_matrices"

const SERVER_MESSAGES_CSV_FILE_NAME = "server.csv"
const CLIENT_MESSAGES_CSV_FILE_NAME = "client.csv"

const MAX_CONNECTIONS_COUNT = 8
const POLLING_FREQUENCY = 100

func getColumns() []string {
	return []string{"client", "time", "kind", "type", "matrixType", "matrixDimension", "id", "status", "message"}
}

type Matrix struct {
	Type      string
	Dimension uint32
}

func (matrix Matrix) getTypeBitSize() uint8 {
	parsed, _ := strconv.ParseUint(matrix.Type[1:], 10, 8)
	return uint8(parsed)
}

func (matrix Matrix) getTypeByteSize() uint8 {
	return matrix.getTypeBitSize() / 8
}

func (matrix Matrix) getByteLength() uint64 {
	return uint64(matrix.Dimension) * uint64(matrix.Dimension) * uint64(matrix.getTypeByteSize())
}

func (matrix Matrix) getRowByteLength() uint32 {
	return matrix.Dimension * uint32(matrix.getTypeByteSize())
}

func (matrix Matrix) String() string {
	return fmt.Sprintf("%dx%d matrix of type %s", matrix.Dimension, matrix.Dimension, matrix.Type)
}

func getMatrixTypes() []string {
	return []string{"u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "f32", "f64"}
}

func getMatrixDimensions() []uint32 {
	return []uint32{10, 100, 1000, 10000}
}

func getMatrixList() *list.List {
	list := list.New()

	for _, mType := range getMatrixTypes() {
		for _, dimension := range getMatrixDimensions() {
			list.PushBack(Matrix{Type: mType, Dimension: dimension})
		}
	}

	return list
}

func getFileName(matrix Matrix) string {
	return fmt.Sprintf("%s_%d_matrix", matrix.Type, matrix.Dimension)
}

func getTestFilePath(matrix Matrix) string {
	relativePath := fmt.Sprintf("%s/%s", TEST_MATRICES_FOLDER, getFileName(matrix))
	path, _ := filepath.Abs(relativePath)
	return path
}

func getDownloadedFilePath(daemonId uint16, matrix Matrix) string {
	relativePath := fmt.Sprintf("%s/%s/%d/%s", CLIENT_FOLDER, DOWNLOADS_FOLDER_NAME, daemonId, getFileName(matrix))
	path, _ := filepath.Abs(relativePath)
	return path
}

func jsonExtractValue(json string, key string) (value string) {
	pattern := fmt.Sprintf(`"%s":"`, key)
	beginIndex := strings.Index(json, pattern)
	if beginIndex != -1 {
		beginIndex += len(pattern)
		endIndex := beginIndex + strings.Index(json[beginIndex:], `"`)
		value = json[beginIndex:endIndex]
	}

	return
}

func runCommand(commandString string, objects chan string) (id uint64, completed bool, err error) {
	tokens := strings.Split(commandString, " ")
	cmd := exec.Command(CLIENT_EXEC, tokens...)
	cmd.Dir = CLIENT_FOLDER
	cmd.Stderr = os.Stderr

	out, err := cmd.Output()
	if err != nil {
		err = fmt.Errorf("Error running command %s: %s\n", commandString, err)
		return
	}

	if len(out) == 0 {
		return
	}

	outStr := string(out)
	tokens = strings.Split(outStr, "\n")
	l := tokens[0]
	r := tokens[1]

	if l == "" || r == "" {
		fmt.Fprintf(os.Stderr, "command %s invalid output: %s %s", commandString, l, r)
	}

	str := jsonExtractValue(r, "id")
	if str != "" {
		id, _ = strconv.ParseUint(str, 10, 64)
	}

	completed = jsonExtractValue(r, "status") == "completed"

	objects <- l
	objects <- r

	return id, completed, nil
}

func handleClient(clientId uint16, daemonId uint16, objects chan string) {
	fmt.Printf("Client %d: initialized\n", clientId)

	queue := getMatrixList()
	for queue.Len() > 0 {
		m := queue.Front()
		queue.Remove(m)
		matrix := m.Value.(Matrix)

		mType := matrix.Type
		mDimension := matrix.Dimension

		fmt.Printf("Client %d: running reserve for a %s\n", clientId, matrix)
		id, _, err := runCommand(fmt.Sprintf("--id=%d --command=reserve --type=%s --dimension=%d --file=%s",
			daemonId, mType, mDimension, getTestFilePath(matrix)), objects)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Client %d: Error running reserve for a %s: %s", clientId, matrix, err)
			os.Exit(1)
		}

		if id == 0 {
			fmt.Printf("Client %d: The server's memory is full. Moving the %s to the end of the queue", clientId, matrix)
			queue.PushBack(matrix)
			continue
		}

		fmt.Printf("Client %d: running calc for the %s (job id %d)\n",
			clientId, matrix, id)
		_, _, err = runCommand(fmt.Sprintf("--id=%d --command=calc --job-id=%d",
			daemonId, id), objects)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Client %d: Error running calc for the %s (job id %d): %s",
				clientId, matrix, id, err)
			os.Exit(1)
		}

		fmt.Printf("Client %d: running poll for the %s (job id %d)\n", clientId, matrix, id)
		pollingFrequency := time.Millisecond * POLLING_FREQUENCY
		for {
			_, completed, err := runCommand(fmt.Sprintf("--id=%d --command=poll --job-id=%d", daemonId, id), objects)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Client %d: Error running poll for the %s (job id %d): %s",
					clientId, matrix, id, err)
				os.Exit(1)
			}

			if completed {
				break
			}

			time.Sleep(pollingFrequency)
		}

	}

	_, _, err := runCommand(fmt.Sprintf("--id=%d --command=close", daemonId), objects)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Client %d: error closing connection: %s", clientId, err)
		os.Exit(1)
	}
	fmt.Printf("Client %d: closed\n", clientId)

	objects <- ""
}

func generateMatrix(matrix Matrix) error {

	file, err := os.Open(getTestFilePath(matrix))
	if err == nil {
		file.Close()
		fmt.Printf("%12s generated previously\n", matrix)
		return nil
	}

	fmt.Printf("%12s generating ", matrix)

	file, err = os.Create(getTestFilePath(matrix))
	if err != nil {
		return err
	}
	defer file.Close()

	buffers := make(chan []uint8)

	go func() {
		rowLen := matrix.getRowByteLength()
		for i := uint32(0); i < matrix.Dimension; i++ {
			buffer := make([]uint8, rowLen)

			_, err := crand.Read(buffer)
			if err != nil {
				fmt.Fprintf(os.Stderr, "\nError generating random bytes for %s at row %d: %s\n", matrix, i, err)
				os.Exit(1)
			}

			buffers <- buffer
		}
	}()

	displayProgressBar := matrix.Dimension >= 100
	if displayProgressBar {
		for i := 0; i < 100; i++ {
			fmt.Print(".")
		}
		fmt.Printf("\r%s generating ", matrix)
	}

	for i := uint32(0); i < matrix.Dimension; i++ {
		if displayProgressBar && 100*i%(matrix.Dimension) == 0 {
			fmt.Print("x")
		}

		file.Write(<-buffers)
	}

	fmt.Print("\033[2K\r") // clear line
	fmt.Printf("%12s generated\n", matrix)

	return nil
}

func redirectToDevNull(pipe io.ReadCloser) {
	io.ReadAll(pipe)
	pipe.Close()
}

func echoWithPrefix(pipe io.ReadCloser, prefix string) {
	defer pipe.Close()
	reader := bufio.NewReader(pipe)
	for {
		line, err := reader.ReadString('\n')
		if err != nil {
			if errors.Is(err, io.EOF) {
				return
			}

			fmt.Fprintf(os.Stderr, "%s Error echoing pipe: %s\n", prefix, err)
			os.Exit(1)
		}

		fmt.Printf("%s: %s", prefix, line)
	}
}

func main() {
	verify := flag.Bool("verify", true, "after testing is complete, verify that the results of matrix transposition by the server are correct")
	flag.Parse()

	fmt.Println("Generating matrices for testing")

	os.Mkdir(TEST_MATRICES_FOLDER, os.ModeDir|os.ModePerm)
	matrixList := getMatrixList()
	for {
		next := matrixList.Front()
		if next == nil {
			break
		}
		matrixList.Remove(next)
		matrix := next.Value.(Matrix)

		err := generateMatrix(matrix)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error generating %s: %s\n", matrix, err)
			os.Exit(1)
		}
	}

	fmt.Println("Starting the server")
	ctx, serverCancel := context.WithCancel(context.Background())
	serverCmd := exec.CommandContext(ctx, SERVER_EXEC)
	serverCmd.Dir = SERVER_FOLDER

	serverStdoutPipe, err := serverCmd.StdoutPipe()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating a stdoutPipe for the server: %s\n", err)
		os.Exit(1)
	}

	serverStderrPipe, err := serverCmd.StderrPipe()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating a stderrPipe for the server: %s\n", err)
		os.Exit(1)
	}

	serverCmd.Start()

	serverStdoutScanner := bufio.NewScanner(serverStdoutPipe)
	serverStdoutScanner.Scan()
	line := serverStdoutScanner.Text()
	go echoWithPrefix(serverStderrPipe, "Server")

	serverPortStr := jsonExtractValue(line, "port")
	parsed, _ := strconv.ParseUint(serverPortStr, 10, 16)
	serverPort := uint16(parsed)
	fmt.Println("The server is running on port", serverPort)

	fmt.Printf("Starting %d concurrent connections\n", MAX_CONNECTIONS_COUNT)
	daemonIds := make([]uint16, MAX_CONNECTIONS_COUNT)
	clientIds := make([]uint16, MAX_CONNECTIONS_COUNT)
	for i := 0; i < MAX_CONNECTIONS_COUNT; i++ {
		daemonCmd := exec.Command(CLIENT_EXEC, "--daemon", fmt.Sprintf("--server=127.0.0.1:%d", serverPort))
		daemonCmd.Dir = CLIENT_FOLDER

		daemonStdoutPipe, err := daemonCmd.StdoutPipe()
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error creating a stdoutPipe for a daemon: %s\n", err)
			serverCancel()
			os.Exit(1)
		}

		daemonStderrPipe, err := daemonCmd.StderrPipe()
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error creating a stderrPipe for a daemon: %s\n", err)
			serverCancel()
			os.Exit(1)
		}

		daemonCmd.Start()

		daemonStdoutReader := bufio.NewReader(daemonStdoutPipe)
		for j := 0; j < 2; j++ {
			line, err := daemonStdoutReader.ReadString('\n')
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error reading daemon stdout: %s\n", err)
				serverCancel()
				os.Exit(1)
			}

			portStr := jsonExtractValue(line, "port")
			port, _ := strconv.ParseUint(portStr, 10, 16)
			kind := jsonExtractValue(line, "kind")

			if kind == "dial" {
				clientIds[i] = uint16(port)
			} else if kind == "listen" {
				daemonIds[i] = uint16(port)
			} else {
				fmt.Fprintf(os.Stderr, "Daemon init error: unknown kind: %s\n", kind)
				serverCancel()
				os.Exit(1)
			}
		}

		go redirectToDevNull(daemonStdoutPipe)
		go echoWithPrefix(daemonStderrPipe, fmt.Sprintf("Client %d", clientIds[i]))
	}

	serverMessagesCSV, err := os.Create(SERVER_MESSAGES_CSV_FILE_NAME)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating file %s: %s\n", SERVER_MESSAGES_CSV_FILE_NAME, err)
		serverCancel()
		os.Exit(1)
	}
	serverMessagesCSV.WriteString(strings.Join(getColumns(), ",") + "\n")

	runningConnections := MAX_CONNECTIONS_COUNT
	clientMessagesCSV, err := os.Create(CLIENT_MESSAGES_CSV_FILE_NAME)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating file %s: %s\n", CLIENT_MESSAGES_CSV_FILE_NAME, err)
		serverCancel()
		os.Exit(1)
	}
	clientMessagesCSV.WriteString(strings.Join(getColumns(), ",") + "\n")

	parseAndWrite := func(obj string, csv *os.File) {
		var parsedObj map[string]string
		json.Unmarshal([]byte(obj), &parsedObj)

		line := ""
		len := len(getColumns())
		for i, column := range getColumns() {
			value, ok := parsedObj[column]
			if ok {
				line += value
			}

			if i < len-1 {
				line += ","
			} else {
				line += "\n"
			}
		}

		csv.WriteString(line)
	}

	go func() {
		for {
			available := serverStdoutScanner.Scan()
			if !available {
				serverStdoutPipe.Close()
				break
			}

			line := serverStdoutScanner.Text()
			parseAndWrite(line, serverMessagesCSV)
		}
	}()

	clientObjectChannel := make(chan string)
	for i := 0; i < MAX_CONNECTIONS_COUNT; i++ {
		go handleClient(clientIds[i], daemonIds[i], clientObjectChannel)
	}

	for runningConnections > 0 {
		obj := <-clientObjectChannel

		if obj == "" {
			runningConnections -= 1
		} else {
			parseAndWrite(obj, clientMessagesCSV)
		}
	}
	clientMessagesCSV.Close()
	close(clientObjectChannel)

	fmt.Println("Terminating the server")
	serverCancel()
	serverCmd.Wait()

	if !*verify {
		return
	}

	fmt.Println("Verifying correctness of the transpositions")
	matrixList = getMatrixList()
	for {
		element := matrixList.Front()
		if element == nil {
			break
		}
		matrixList.Remove(element)
		matrix := element.Value.(Matrix)
		bytesLen := matrix.getByteLength()

		readBytesFrom := func(filePath string) []uint8 {
			bytes, err := ioutil.ReadFile(filePath)

			if err != nil {
				fmt.Fprintf(os.Stderr, "Error reading file %s: %s\n", filePath, err)
				os.Exit(1)
			}

			if uint64(len(bytes)) != bytesLen {
				fmt.Fprintf(os.Stderr, "The size of %s %d does not match the size of the %s %d\n",
					getTestFilePath(matrix), len(bytes), matrix, bytesLen)
				os.Exit(1)
			}

			return bytes
		}

		typeSize := uint32(matrix.getTypeByteSize())
		getSlice := func(bytes []uint8, i uint32, j uint32) []uint8 {
			begin := (i*matrix.Dimension + j) * typeSize
			end := begin + typeSize
			return bytes[begin:end]
		}

		sliceToString := func(bytes []uint8) string {
			strs := make([]string, len(bytes))
			for i, b := range bytes {
				strs[i] = strconv.FormatUint(uint64(b), 10)
			}

			return fmt.Sprintf("[%s]", strings.Join(strs, " "))
		}

		origFilePath := getTestFilePath(matrix)
		origBytes := readBytesFrom(origFilePath)
	Outer:
		for _, id := range clientIds {
			transposedBytes := readBytesFrom(getDownloadedFilePath(id, matrix))

			fmt.Printf("%12s verifying ", matrix)

			displayProgressBar := matrix.Dimension >= 100
			if displayProgressBar {
				for i := 0; i < 100; i++ {
					fmt.Print(".")
				}
				fmt.Printf("\r%12s verifying ", matrix)
			}

			for i := uint32(0); i < matrix.Dimension; i++ {
				for j := uint32(0); j <= i; j++ {
					origSlice := getSlice(origBytes, i, j)
					transposedSlice := getSlice(transposedBytes, j, i)
					if !bytes.Equal(origSlice, transposedSlice) {
						fmt.Printf("\033[2K\r%12s error: orig(%d, %d)=%s != transposed(%d, %d)=%s\n",
							matrix, i, j, sliceToString(origSlice), j, i, sliceToString(transposedSlice))
						continue Outer
					}

					if displayProgressBar && 100*(uint64(i)*uint64(matrix.Dimension)+uint64(j))%(bytesLen) == 0 {
						fmt.Print("x")
					}
				}
			}

			fmt.Printf("\033[2K\r%12s verified\n", matrix)
		}
	}
}
