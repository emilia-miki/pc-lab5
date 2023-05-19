public class GetStatusCommand : Command
{
	static GetStatusCommand? instance;
	public static GetStatusCommand GetInstance() { 
		if (instance == null)
		{
			instance = new GetStatusCommand();
		}

		return instance;
	}

	static GetStatusCommand() {}
	private GetStatusCommand()
	{
		encoding = Constants.Instance.GetCommandEncodings()[GetType()];
	}

	private enum Status
	{
		NoData,
		Ready,
		Running,
		Completed,
	}

	private Status GetStatus()
	{
		var value = bytes[1];
		if (value >= Enum.GetValues<Status>().Length)
		{
			throw new Exception(
				$"Unknown server response! Expected 0..2 for status, " +
				"but received {bytes[1]}");
		}

		return (Status) value;
	}

    protected override void ParseTokens()
    {
		if (tokens.Length > 1)
		{
			throw new Exception("There can be no arguments provided to this command");
		}
    }

	private byte index;

	private void SetIndex()
	{
		bytes[1] = index;
	}

    protected override void PrepareRequestMessage()
    {
		bufferSize = 2;

		index = state.GetStatusGet();

		SetCommand();
		SetIndex();
    }

    protected override void HandleResponseMessage()
    {
        base.HandleResponseMessage();

		var status = GetStatus();
		switch (status)
		{
			case Status.NoData:
				Console.WriteLine(
					"You haven't provided a matrix for this index!");

				state.GetStatusSet(index);
				return;
			case Status.Ready:
				Console.WriteLine(
					"The job is ready to start");

				state.GetStatusSet(index);
				return;
			case Status.Running:
				Console.WriteLine("The calculation is running.");

				state.GetStatusSet(index);
				return;
			case Status.Completed:
				Console.WriteLine(
					"Calculation complete! Downloading the result...");
				break;
		}	

		var spec = state.GetMatrixSpecs(index);
		var buffer = new byte[spec.BufferLength];
		var bufferIndex = 0;

		receivedCount -= 2;
		new Span<byte>(bytes, 2, receivedCount)
			.CopyTo(new Span<byte>(buffer, bufferIndex, receivedCount));
		bufferIndex += receivedCount;

		while (bufferIndex < buffer.Length)
		{
			ReceiveResponseMessage();

			new Span<byte>(bytes, 0, receivedCount)
				.CopyTo(new Span<byte>(buffer, bufferIndex, receivedCount));
			bufferIndex += receivedCount;
		}

		var transposedMatrix = Matrix.FromBytes(spec.TypeSize, spec.Type, spec.Dimension, buffer);

		const string downloadsDir = "downloaded_matrices";
		if (!Directory.Exists(downloadsDir))
		{
			Directory.CreateDirectory(downloadsDir);
		}

		var rand = new Random();
        var filename = downloadsDir + "/" +
			(((DateTimeOffset) DateTime.UtcNow).ToUnixTimeMilliseconds()) + rand.Next() + ".csv";

		transposedMatrix.ToFile(filename);

		Console.Write($"The transposed matrix has been downloaded to file {filename}.");
		if (Console.IsInputRedirected)
		{
			Console.WriteLine();
		}
		else
		{
			Console.Write(
				" Do you want to view it in terminal? (y/N) ");
			var input = Console.ReadLine()!;
			if (input.Trim().ToLower() == "y")
			{
				transposedMatrix.ToCli();
			}
		}
    }
}
