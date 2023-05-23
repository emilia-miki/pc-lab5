package main

import (
	"bufio"
	"flag"
	"fmt"
	"net"
	"os"
	"strings"

	"client/command"
	"client/constants"
	"client/matrix/mtype"
	"client/message"
	messageType "client/message/mtype"
)

func runDaemonMainLoop(server string) error {
	// start a listener
	listener, err := net.Listen("tcp", ":0")
	if err != nil {
		return fmt.Errorf("error starting a listener: %s", err)
	}
	id := uint16(listener.Addr().(*net.TCPAddr).Port)

	// create a file named after the used port for the clients to easily find this listener
	daemonFileName := fmt.Sprintf("%s/%d", constants.RUNNING_DAEMONS_FOLDER, id)
	f, err := os.Create(daemonFileName)
	if err != nil {
		return err
	}
	f.Close()

	// delete the file when the daemon exits
	defer os.Remove(daemonFileName)

	// establish connection to the server
	serverConnection, err := net.Dial("tcp", server)
	if err != nil {
		return fmt.Errorf("daemon %d error: error connecting to the server: %s", id, err)
	}
	defer serverConnection.Close()
	clientId := uint16(serverConnection.LocalAddr().(*net.TCPAddr).Port)

	// notify the client about ports
	fmt.Printf("{\"kind\":\"listen\",\"port\":\"%d\"}\n", id)
	fmt.Printf("{\"kind\":\"dial\",\"port\":\"%d\"}\n", clientId)

	for {
		// get a client connection
		connection, err := listener.Accept()
		if err != nil {
			return fmt.Errorf("daemon %d error: error accepting connection: %s", clientId, err)
		}
		defer connection.Close()

		// read one line
		scanner := bufio.NewScanner(connection)
		scanner.Scan()
		line := scanner.Text()

		// parse the command
		args := strings.Split(line, " ")
		cmd, err := command.CommandFromString(args[0])
		if err != nil {
			return err
		}

		// prepare the appropriate request
		var request message.Request
		switch cmd {
		case command.Reserve:
			request = message.NewRequest(clientId, messageType.Reserve, map[string]string{
				"matrixType":      args[1],
				"matrixDimension": args[2],
				"file":            args[3],
			})
			break
		case command.Calc:
			request = message.NewRequest(clientId, messageType.Calc, map[string]string{
				"index":       args[1],
				"threadCount": args[2],
			})
			break
		case command.Poll:
			request = message.NewRequest(clientId, messageType.Poll, map[string]string{
				"index": args[1],
			})
			break
		case command.Close: // terminates the daemon
			return nil
		}

		response, err := request.Execute(serverConnection)
		if err != nil {
			return fmt.Errorf("Daemon %d error: error processing request: %s", clientId, err)
		}

		jsonRequest := request.JsonString()
		fmt.Fprintln(connection, jsonRequest)

		jsonResponse := response.JsonString()
		fmt.Fprintln(connection, jsonResponse)
	}
}

func main() {
	// initialize command-line arguments
	daemon := flag.Bool("daemon", false, "starts a daemon that communicates with the server")
	id := flag.Uint("id", 0, "ID of the daemon to connect to")
	listDaemons := flag.Bool("list-daemons", false, "lists all running daemons")
	server := flag.String("server", constants.DEFAULT_SERVER_CONNECTION_STRING, "the IP address of the server")

	// non-daemon only
	commandStr := flag.String("command", "", "the command to send to the server")

	// sendData command arguments
	mTypeStr := flag.String("type", "", "the matrix type")
	mDimension := flag.String("dimension", "", "the dimension of the matrix")
	mFileName := flag.String("file", "", "the file to read the matrix from")

	// startCalculation, getStatus command arguments
	index := flag.Uint("index", 0, "the index of a registered job")

	// getStatus command arguments
	threadCount := flag.Uint("threadCount", 0,
		"the number of threads for the server to process your matrix with")

	flag.Parse()

	if *listDaemons {
		dir, err := os.Open(constants.RUNNING_DAEMONS_FOLDER)
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		defer dir.Close()

		daemonFileInfos, err := dir.ReadDir(-1)
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}

		daemonIds := make([]string, len(daemonFileInfos))
		for i, fileInfo := range daemonFileInfos {
			daemonIds[i] = fileInfo.Name()
		}
		fmt.Println(strings.Join(daemonIds, "\n"))
		return
	}

	// run the daemon instead of the other stuff if requested
	if *daemon {
		// check if all required folders exist
		os.Mkdir(constants.DOWNLOADS_FOLDER, os.ModeDir|os.ModePerm)
		os.Mkdir(constants.RUNNING_DAEMONS_FOLDER, os.ModeDir|os.ModePerm)

		err := runDaemonMainLoop(*server)
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}

		return
	}

	// parse the arguments
	if *id == 0 {
		fmt.Fprintf(os.Stderr, "You must specify the daemon ID'\n")
		os.Exit(1)
	}

	// parse command arguments
	cmd, err := command.CommandFromString(*commandStr)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	switch cmd {
	case command.Reserve:
		_, err := mtype.FromString(*mTypeStr)
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}

		if *mFileName == "" {
			fmt.Fprintf(os.Stderr, "Invalid file argument: must be a non-empty string\n")
			os.Exit(1)
		}

		break
	case command.Calc:
		if *threadCount == 0 {
			fmt.Fprintf(os.Stderr, "Invalid threadCount argument: must be a strictly positive integer.\n")
			os.Exit(1)
		}

		fallthrough
	case command.Poll:
		if *index == 0 {
			fmt.Fprintf(os.Stderr, "Invalid index argument: must be a strictly positive integer.\n")
			os.Exit(1)
		}
		break
	}

	// connect to the daemon
	connection, err := net.Dial("tcp", fmt.Sprintf("127.0.0.1:%d", *id))
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error connecting to the daemon at port %d: %s\n", *id, err)
		os.Exit(1)
	}
	defer connection.Close()

	// send the command
	switch cmd {
	case command.Reserve:
		_, err = fmt.Fprintln(connection, *commandStr, *mTypeStr, *mDimension, *mFileName)
		break
	case command.Calc:
		_, err = fmt.Fprintln(connection, *commandStr, *index, *threadCount)
		break
	case command.Poll:
		_, err = fmt.Fprintln(connection, *commandStr, *index)
		break
	case command.Close:
		_, err = fmt.Fprintln(connection, *commandStr)
		break
	}

	if err != nil {
		fmt.Fprintf(os.Stderr, "Error sending command to the daemon at port %d: %s\n", *id, err)
		os.Exit(1)
	}

	// echo the daemon's output
	// we expect 1 line for request and 1 for response
	if cmd != command.Close {
		scanner := bufio.NewScanner(connection)
		for i := 0; i < 2; i++ {
			scanner.Scan()
			fmt.Println(scanner.Text())
		}
	}
}
