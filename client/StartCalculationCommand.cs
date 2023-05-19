public class StartCalculationCommand : Command
{
	Stack<byte> indexStack = new();

	static StartCalculationCommand instance = new StartCalculationCommand();
	public static StartCalculationCommand GetInstance()
	{
		if (instance == null)
		{
			instance = new StartCalculationCommand();
		}

		return instance;
	}

	static StartCalculationCommand() {}
	StartCalculationCommand()
	{
		encoding = Constants.Instance.GetCommandEncodings()[GetType()];
	}

    protected override void ParseTokens()
    {
		threadCount = 0;

		if (tokens.Length > 2)
		{
			throw new Exception(
				$"Invalid number of arguments! Required 0 or 1, but received {tokens.Length - 1}");
		}

		if (tokens.Length == 2)
		{
			if (!byte.TryParse(tokens[1], out threadCount))
			{
				throw new Exception("Invalid thread count! The value must be a positive integer under 255");
			}
		}
    }
    
    private byte threadCount = 0;

	protected void SetThreadCount()
	{
		bytes[2] = threadCount;
	}

	byte index = 0;

	void SetIndex()
	{
		bytes[1] = index;
	}

    protected override void PrepareRequestMessage()
    {
		bufferSize = 3;

		index = state.StartCalculationGet();

		SetCommand();
		SetIndex();
		SetThreadCount();
    }

    protected override void HandleResponseMessage()
    {
		base.HandleResponseMessage();

		state.StartCalculationSet(index);

		Console.WriteLine("Calculation started!");
    }
}
