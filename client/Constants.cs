public sealed class Constants
{
    public const string ADDR = "127.0.0.1";
    public const UInt16 PORT = 3333;

    static Constants instance = new Constants();
    public static Constants Instance { get => instance; }

    IReadOnlyDictionary<Type, byte>? commandEncodings;
    IReadOnlyDictionary<string, Command>? commandsByString;
    IReadOnlyDictionary<Command, string>? stringsByCommand;

    public readonly Type[] Commands;

    public IReadOnlyDictionary<Type, byte> GetCommandEncodings()
    {
        if (commandEncodings == null) {
            commandEncodings = Commands
                .Select((type, index) => new { Type = type, Index = index })
                .ToDictionary(item => item.Type, item => (byte)item.Index);
        }

        return commandEncodings;
    }

    public IReadOnlyDictionary<string, Command> GetCommandsByString()
    {
        if (commandsByString == null) {
            commandsByString = new Dictionary<string, Command>()
            {
                { "send_data", (Command) typeof(SendDataCommand).GetMethod("GetInstance")!.Invoke(null, null)! },
                { "start_calculation", (Command) typeof(StartCalculationCommand).GetMethod("GetInstance")!.Invoke(null, null)! },
                { "get_status", (Command) typeof(GetStatusCommand).GetMethod("GetInstance")!.Invoke(null, null)! },
            };
        }

        return commandsByString;
    }

    public IReadOnlyDictionary<Command, string> StringsByCommand()
    {
        if (stringsByCommand == null) {
            stringsByCommand = GetCommandsByString()
                .ToDictionary(pair => pair.Value, pair => pair.Key);
        }

        return stringsByCommand;
    }

    static Constants() {}
    private Constants()
    {
        // Here the order is also important, but only because it has to
        // match the server-side specs.
        Commands = new[]
        {
            typeof(SendDataCommand),
            typeof(StartCalculationCommand),
            typeof(GetStatusCommand),
        };
    }
}
