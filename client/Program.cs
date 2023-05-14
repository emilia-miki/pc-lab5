using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Globalization;

const string ADDR = "127.0.0.1";
const int PORT = 8001;

byte? latestIndex = null;

var types = new Dictionary<Type, (int, object)>()
{
    { typeof(bool), (0, (Func<string, bool>) Convert.ToBoolean) },
    { typeof(byte), (5, (Func<string, byte>) Convert.ToByte) },
    { typeof(UInt16), (2, (Func<string, UInt16>) Convert.ToUInt16) },
    { typeof(UInt32), (3, (Func<string, UInt32>) Convert.ToUInt32) },
    { typeof(UInt64), (4, (Func<string, UInt32>) Convert.ToUInt32) },
    { typeof(sbyte), (1, (Func<string, sbyte>) Convert.ToSByte) },
    { typeof(Int16), (6, (Func<string, Int16>) Convert.ToInt16) },
    { typeof(Int32), (7, (Func<string, Int32>) Convert.ToInt32) },
    { typeof(Int64), (8, (Func<string, Int32>) Convert.ToInt32) },
    { typeof(float), (9, (Func<string, float>) Convert.ToSingle) },
    { typeof(double), (10, (Func<string, double>) Convert.ToDouble) },
};

Console.WriteLine("Welcome to the matrix transposer! Connecting to a server...");

var client = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
client.Connect(new IPEndPoint(IPAddress.Parse(ADDR), PORT));

Console.WriteLine($"Connected to {ADDR}:{PORT}.");
Console.WriteLine("The client works by accepting commands and sending them" +
                  "to the server. You can always get the list of commands by" +
                  "typing help.");

while (true)
{
    Console.Write("> ");
    var input = Console.ReadLine();
    if (input == null)
    {
        continue;
    }

    var tokens = input.Trim().Split(' ');
    switch (tokens[0])
    {
        case "help":
            Console.WriteLine("List of commands:");
            Console.WriteLine("help - get this list");
            Console.WriteLine("send_data FILENAME - send a matrix from the " +
                              "file FILENAME to the server. The file must " +
                              "be in the CSV format without column headers. " +
                              "If no FILENAME is provided, you will be " +
                              "prompted to enter the matrix manually, " +
                              "line by line, with values separated by spaces.");
            Console.WriteLine("start_calculation INDEX - send a signal to " +
                              "the server to start transposing the matrix " +
                              "at index INDEX. If no INDEX provided, this " +
                              "will automatically choose the latest matrix " +
                              "you have sent to the server.");
            Console.WriteLine("get_status INDEX - get calculation status " +
                              "for the matrix at index INDEX. If it is " +
                              "complete, the server will also send back the " +
                              "result. If no INDEX provided, this will " +
                              "automatically choose the latest matrix you " +
                              "have sent to the server.");
            break;
        case "send_data":
            if (tokens.Length > 2)
            {
                Console.WriteLine("Too many arguments!");
                break;
            }

            IEnumerable<string> lines;
            uint matrixDimension;

            if (tokens.Length == 2)
            {
                if (!File.Exists(tokens[1]))
                {
                    Console.WriteLine($"File not found: {tokens[1]}");
                }

                lines = File.ReadLines(tokens[1]).Where(line => line != string.Empty);
                matrixDimension = (uint)lines.Count();
            }
            else
            {
                var linesList = new List<string>();
                lines = linesList;

                var line = Console.ReadLine();

                if (line == null)
                {
                    Console.WriteLine("Could not read line");
                    break;
                }

                if (line == string.Empty)
                {
                    Console.WriteLine("Empty line read. Command canceled.");
                    break;
                }

                linesList.Add(line);

                matrixDimension = (uint) line.Split(' ').Length;

                for (var i = 0; i < matrixDimension - 1; i++)
                {
                    line = Console.ReadLine();
                    if (line == null)
                    {
                        Console.WriteLine("Could not read line");
                        break;
                    }

                    if (line == string.Empty)
                    {
                        Console.WriteLine("Empty line read. Command canceled.");
                        break;
                    }

                    linesList.Add(line);
                }
            }

            if (lines.Count() == 0)
            {
                Console.WriteLine("Empty matrix received. Command canceled.");
                break;
            }

            var strMatrix = lines.SelectMany(line => line.Split(',')).ToList();

            if (strMatrix.Count() != matrixDimension * matrixDimension)
            {
                Console.WriteLine($"Invalid file format");
                break;
            }

            byte[]? message = null;
            foreach (var (type, (typeEncoding, converter)) in types)
            {
                byte[] msg;
                try
                {
                    var value = ((Func<string, object>)converter)(strMatrix[0].Trim());
                    var getBytesMethod = typeof(BitConverter).GetMethod("GetBytes", new[] { type })!;
                    var bytes = (byte[])getBytesMethod.Invoke(null, new[] { value })!;
                    var typeSize = bytes.Length;

                    msg = new byte[1 + 4 + 1 + matrixDimension * matrixDimension * typeSize];

                    for (var i = 0; i < matrixDimension * matrixDimension; i++)
                    {
                        value = ((Func<string, object>)converter)(strMatrix[i].Trim());
                        getBytesMethod = typeof(BitConverter).GetMethod("GetBytes", new[] { type })!;
                        bytes = (byte[])getBytesMethod.Invoke(null, new[] { value })!;
                        bytes.CopyTo(msg, 6 + i * typeSize);
                    }
                }
                catch
                {
                    continue;
                }

                msg[0] = 0;
                BitConverter.GetBytes(matrixDimension).CopyTo(msg, 1);
                msg[5] = (byte) typeEncoding;

                message = msg;
                break;
            }

            if (message == null)
            {
                Console.WriteLine("Invalid matrix type. Command canceled.");
                break;
            }

            var sent = client.Send(message);
            if (sent != message.Length)
            {
                Console.WriteLine(
                    $"Error sending the message! Only {sent} bytes sent " +
                    "out of {message.Length}.");
            }

            var received = client.Receive(message);
            if (received == 0)
            {
                Console.WriteLine("Invalid server response.");
                break;
            }

            switch (message[0])
            {
                case 0:
                    latestIndex = message[1];
                    Console.WriteLine(
                        "The matrix was received by the server. It is stored " +
                        $"under index {message[1]}.");
                    break;
                case 1:
                    Console.WriteLine("Server error response: " +
                        Encoding.UTF8.GetString(
                            new Span<byte>(message, 1, message.Length - 1)));
                    break;
                default:
                    Console.WriteLine("Unknown server response.");
                    break;
            }
            break;
        case "start_calculation":
            if (tokens.Length > 2)
            {
                Console.WriteLine("Too many arguments!");
                break;
            }

            byte index;
            if (tokens.Length == 2)
            {
                var parsed = byte.TryParse(tokens[1], out index);
                if (!parsed)
                {
                    Console.WriteLine(
                        $"Couldn't parse the index argument: {tokens[1]}. " +
                        "Command canceled.");
                    break;
                }
            }
            else
            {
                if (latestIndex == null)
                {
                    Console.WriteLine(
                        "You can't start a calculation, because you haven't " +
                        "sent a matrix to the server.");
                    break;
                }

                index = latestIndex.Value;
            }

            message = new byte[2];
            message[0] = 1;
            message[1] = index;

            sent = client.Send(message);
            if (sent != message.Length)
            {
                Console.WriteLine(
                    $"Error sending the message! Only {sent} bytes sent " +
                    "out of {message.Length}.");
            }

            received = client.Receive(message);
            if (received == 0)
            {
                Console.WriteLine("Invalid server response.");
                break;
            }

            switch (message[0])
            {
                case 0:
                    Console.WriteLine("Calculation started!");
                    break;
                case 1:
                    Console.WriteLine("Server error response: " +
                        Encoding.UTF8.GetString(
                            new Span<byte>(message, 1, message.Length - 1)));
                    break;
                default:
                    Console.WriteLine("Unknown server response.");
                    break;
            }
            break;
        case "get_status":
            if (tokens.Length > 2)
            {
                Console.WriteLine("Too many arguments!");
                break;
            }

            if (tokens.Length == 2)
            {
                var parsed = byte.TryParse(tokens[1], out index);
                if (!parsed)
                {
                    Console.WriteLine(
                        $"Couldn't parse the index argument: {tokens[1]}. " +
                        "Command canceled.");
                    break;
                }
            }
            else
            {
                if (latestIndex == null)
                {
                    Console.WriteLine(
                        "You can't start a calculation, because you haven't " +
                        "sent a matrix to the server.");
                    break;
                }

                index = latestIndex.Value;
            }

            message = new byte[2];
            message[0] = 1;
            message[1] = index;

            sent = client.Send(message);
            if (sent != message.Length)
            {
                Console.WriteLine(
                    $"Error sending the message! Only {sent} bytes sent " +
                    "out of {message.Length}.");
            }

            received = client.Receive(message);
            if (received == 0)
            {
                Console.WriteLine("Invalid server response.");
                break;
            }

            switch (message[0])
            {
                case 0:
                    switch (message[1])
                    {
                        case 0:
                            Console.WriteLine(
                                "No matrix provided for this index!");
                            break;
                        case 1:
                            Console.WriteLine(
                                "The calculation is running.");
                            break;
                        case 2:
                            Console.WriteLine("Calculation complete! Downloading the result.");
                            var filename = DateTime.UtcNow.ToString(
                                CultureInfo
                                .InvariantCulture
                                .DateTimeFormat
                                .SortableDateTimePattern) + "-matrix.csv";

                            using (var file = File.Open(filename, FileMode.Create))
                            {
                                // TODO: read the matrix from socket
                                // and send to file; handle errors
                            }

                            Console.WriteLine($"The result has been downloaded to file {filename}.");
                            break;
                        default:
                            Console.WriteLine("Unknown server response.");
                            break;
                    }
                    break;
                case 1:
                    Console.WriteLine("Server error response: " +
                        Encoding.UTF8.GetString(
                            new Span<byte>(message, 1, message.Length - 1)));
                    break;
                default:
                    Console.WriteLine("Unknown server response.");
                    break;
            }
            break;
        default:
            Console.WriteLine($"Unknown command: {tokens[0]}. " +
                               "Type help to get the list of commands.");
            break;
    }
}
