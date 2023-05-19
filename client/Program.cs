using System.Net;
using System.Net.Sockets;

Console.WriteLine("Welcome to the matrix transposer!");
Console.WriteLine("Connecting to a server...");

var client = new Socket(
    AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);

var portArgs = args.Where(arg => arg.StartsWith("port="));
var portArgsCount = portArgs.Count();
if (portArgsCount > 1)
{
    throw new Exception(
        $"You can only provide one argument specifying the port");
}

var port = portArgsCount == 0 ? Constants.PORT : int.Parse(portArgs.First());

client.Connect(new IPEndPoint(IPAddress.Parse(Constants.ADDR), port));

Console.WriteLine($"Connected to {Constants.ADDR}:{port}.");
Console.WriteLine(
    "The client works by accepting commands and sending them to the server. " +
    "You can also type help to get the list of commands.");

var state = State.GetInstance();
state.Socket = client;

while (true)
{
    Console.Write("> ");
    var input = Console.ReadLine();
    if (string.IsNullOrWhiteSpace(input))
    {
        continue;
    }

    var tokens = input.Trim().Split(' ');
    if (tokens[0] == "help")
    {
        Console.WriteLine("List of commands:");

        Console.WriteLine("> help");
        Console.WriteLine("get this list");

        Console.WriteLine("> send_data FILENAME");
        Console.WriteLine(
            "send a matrix from the file FILENAME to the server. " +
            "The file must be in the CSV format without column headers. " +
            "If no FILENAME is provided, you will be prompted to enter " +
            "the matrix manually, line by line, with values separated " +
            "by spaces.");

        Console.WriteLine("> start_calculation INDEX");
        Console.WriteLine(
            "send a signal to the server to start transposing the matrix " +
            "at index INDEX. If no INDEX provided, this will automatically " +
            "choose the latest matrix you've sent.");

        Console.WriteLine("> get_status INDEX");
        Console.WriteLine(
            "get calculation status for the matrix at index INDEX. If it is " +
            "completed, the server will also send back the result. If no " +
            "INDEX provided, this command will automatically choose the " +
            "latest matrix you have sent to the server.");

        Console.WriteLine("> exit");
        Console.WriteLine("exits the program.");

        continue;
    }

    if (tokens[0] == "exit")
    {
        client.Close();
        client.Dispose();
        return;
    }

    try
    {
        Command.Run(tokens);
    }
    catch (Exception e)
    {
        Console.WriteLine(e.Message);

        if (e.Message == "The server disconnected")
        {
            return;
        }
    }
}
