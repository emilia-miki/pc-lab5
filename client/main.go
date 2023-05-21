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

func cleanUp(listener net.Listener) {
	err := os.Remove(fmt.Sprintf("%s/%d",
		constants.RUNNING_DAEMONS_FOLDER, listener.Addr().(*net.TCPAddr).Port))

	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func runDaemonMainLoop() error {
	// start a listener
	listener, err := net.Listen("tcp", ":0")
	if err != nil {
		return err
	}
	defer cleanUp(listener)

	// create a file named after the used port for the clients to easily find this listener
	id := uint16(listener.Addr().(*net.TCPAddr).Port)
	f, err := os.Create(fmt.Sprintf("%s/%d", constants.RUNNING_DAEMONS_FOLDER, id))
	if err != nil {
		return err
	}
	f.Close()
	fmt.Printf("{\"kind\":\"listen\",\"port\":%d}\n", id)

	// establish connection to the server
	serverConnection, err := net.Dial("tcp", constants.SERVER_CONNECTION_STRING)
	if err != nil {
		return fmt.Errorf("Error connecting to server: %s.", err)
	}
	defer serverConnection.Close()
	fmt.Printf("{\"kind\":\"dial\",\"port\":%d}\n", serverConnection.LocalAddr().(*net.TCPAddr).Port)

	for {
		// get a client connection
		connection, err := listener.Accept()
		if err != nil {
			return err
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
		case command.SendData:
			request = message.NewRequest(id, messageType.SendData, map[string]string{
				"type": args[1],
				"file": args[2],
			})
			break
		case command.StartCalculation:
			request = message.NewRequest(id, messageType.StartCalculation, map[string]string{
				"index":       args[1],
				"threadCount": args[2],
			})
			break
		case command.GetStatus:
			request = message.NewRequest(id, messageType.GetStatus, map[string]string{
				"index": args[1],
			})
			break
		case command.CloseConnection: // terminates the daemon
			return nil
		}

		// send the request and echo it to the client
		err = request.Send(serverConnection)
		if err != nil {
			return err
		}
		jsonRequest := request.JsonString()
		fmt.Fprintln(connection, jsonRequest)
		fmt.Println(jsonRequest) // echo

		// get a response and echo it to the client
		response, err := message.Receive(id, serverConnection)
		if err != nil {
			return err
		}
		jsonResponse := response.JsonString()
		fmt.Fprintln(connection, jsonResponse)
		fmt.Println(jsonResponse) // echo

		// close the connection and wait for the next client connection (deferred)
	}
}

func main() {
	// initialize command-line arguments
	daemon := flag.Bool("daemon", false, "starts a daemon that communicates with the server")
	id := flag.Uint("id", 0, "ID of the daemon to connect to")
	listDaemons := flag.Bool("list-daemons", false, "lists all running daemons")

	// non-daemon only
	commandStr := flag.String("command", "", "the command to send to the server")

	// sendData command arguments
	mTypeStr := flag.String("type", "", "the matrix type")
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

		list, err := dir.ReadDir(-1)
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}

		str := ""
		for _, fileInfo := range list {
			str += fmt.Sprintf("%s ", fileInfo.Name())
		}
		fmt.Println(str[:len(str)-1])
		return
	}

	// run the daemon instead of the other stuff if requested
	if *daemon {
		// check if all required folders exist
		err := os.Mkdir(constants.DOWNLOADS_FOLDER, os.ModeDir|os.ModePerm)
		if err != nil && !strings.HasSuffix(err.Error(), " file exists") {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}

		err = os.Mkdir(constants.RUNNING_DAEMONS_FOLDER, os.ModeDir|os.ModePerm)
		if err != nil && !strings.HasSuffix(err.Error(), " file exists") {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}

		err = runDaemonMainLoop()
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}

		return
	}

	// parse the arguments
	if *id == 0 {
		fmt.Fprintf(os.Stderr, "You must specify a daemon ID to connect to")
		os.Exit(1)
	}

	// parse command arguments
	cmd, err := command.CommandFromString(*commandStr)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	switch cmd {
	case command.SendData:
		_, err := mtype.FromString(*mTypeStr)
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}

		if *mFileName == "" {
			fmt.Fprintf(os.Stderr, "Invalid file argument: must be a non-empty string.\n")
			os.Exit(1)
		}

		break
	case command.StartCalculation:
		if *threadCount == 0 {
			fmt.Fprintf(os.Stderr, "Invalid threadCount argument: must be a strictly positive integer.\n")
			os.Exit(1)
		}

		fallthrough
	case command.GetStatus:
		if *index == 0 {
			fmt.Fprintf(os.Stderr, "Invalid index argument: must be a strictly positive integer.\n")
			os.Exit(1)
		}
		break
	case command.CloseConnection:
		break
	}

	// connect to the daemon
	connection, err := net.Dial("tcp", fmt.Sprintf("127.0.0.1:%d", *id))
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	defer connection.Close()

	// send the command
	switch cmd {
	case command.SendData:
		fmt.Fprintln(connection, *commandStr, *mTypeStr, *mFileName)
		break
	case command.StartCalculation:
		fmt.Fprintln(connection, *commandStr, *index, *threadCount)
		break
	case command.GetStatus:
		fmt.Fprintln(connection, *commandStr, *index)
		break
	case command.CloseConnection:
		fmt.Fprintln(connection, *commandStr)
		break
	}

	// echo the daemon's output
	scanner := bufio.NewScanner(connection)
	for i := 0; i < 2; i++ {
		scanner.Scan()
		fmt.Println(scanner.Text())
	}
}
