public sealed class Constants
{
    public const string ADDR = "127.0.0.1";
    public const UInt16 PORT = 3333;

    // Don't touch the order! It is very important.
    // When trying to parse a matrix from string, the program
    // will go over types in this order, so it will try
    // parsing into smaller types first, and then go to
    // larger ones.
    public static readonly IReadOnlyList<Type> Types = new[]
    {
        typeof(bool),
        typeof(byte),
        typeof(UInt16),
        typeof(UInt32),
        typeof(UInt64),
        typeof(sbyte),
        typeof(Int16),
        typeof(Int32),
        typeof(Int64),
        typeof(float),
        typeof(double),
    };

    public static readonly IReadOnlyDictionary<Type, byte> TypeEncodings =
		Types
		.Select((type, index) => new { Index = index, Type = type })
	    .ToDictionary(item => item.Type, item => (byte) item.Index);

    // Here the order is also important, but only because it has to
    // match the server-side specs.
    public static Type[] Commands = new[]
    {
        typeof(SendDataCommand),
        typeof(StartCalculationCommand),
        typeof(GetStatusCommand),
    };

    public static readonly IReadOnlyDictionary<Type, byte> CommandEncodings =
        Commands
        .Select((type, index) => new { Type = type, Index = index })
        .ToDictionary(item => item.Type, item => (byte) item.Index);

    public static readonly IReadOnlyDictionary<string, Type> CommandsByString =
        new Dictionary<string, Type>()
        {
            { "send_data", typeof(SendDataCommand) },
            { "start_calculation", typeof(StartCalculationCommand) },
            { "get_status", typeof(GetStatusCommand) },
        };

    public static readonly IReadOnlyDictionary<Type, string> StringsByCommand =
        CommandsByString
        .ToDictionary(pair => pair.Value, pair => pair.Key);
}
