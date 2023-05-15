using System.Globalization;

public class GetStatusCommand : IndexCommand
{
	private static GetStatusCommand instance = new GetStatusCommand();
	public static GetStatusCommand Instance { get => instance; }

	static GetStatusCommand() {}
	private GetStatusCommand()
	{
		encoding = Constants.CommandEncodings[GetType()];
	}

	private enum Status
	{
		NoData,
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

    protected override void HandleResponseMessage()
    {
        base.HandleResponseMessage();

		var status = GetStatus();
		switch (status)
		{
			case Status.NoData:
				Console.WriteLine(
					"You haven't provided a matrix for this index!");
				return;
			case Status.Running:
				Console.WriteLine("The calculation is running.");
				return;
			case Status.Completed:
				Console.WriteLine(
					"Calculation complete! Downloading the result...");
				break;
		}	

		var matrix = state.GetMatrix(index);
		var bufferSize = matrix.Bytes.Length;
		var buffer = new byte[bufferSize];
		var bufferIndex = 0;

		new Span<byte>(bytes, 2, receivedCount - 2)
		.CopyTo(new Span<byte>(buffer, bufferIndex, receivedCount));
		bufferIndex += receivedCount;

		while (bufferIndex < buffer.Length)
		{
			ReceiveResponseMessage();

			
			new Span<byte>(bytes, 0, receivedCount)
			.CopyTo(new Span<byte>(buffer, bufferIndex, receivedCount));
			bufferIndex += receivedCount;
		}

		var transposedMatrix = Matrix.FromBytes(buffer, matrix.Type, matrix.Dimension);

        var filename = DateTime.UtcNow.ToString(
            CultureInfo
            .InvariantCulture
            .DateTimeFormat
            .SortableDateTimePattern) + "-matrix.csv";

		transposedMatrix.ToFile(filename);

		Console.Write(
			$"The transposed matrix has been downloaded to file {filename}. " +
			"Do you want to view it in terminal? (y/N) ");
		var input = Console.ReadLine()!;
		if (input.Trim().ToLower() == "y")
		{
			transposedMatrix.ToCli();
		}
    }
}
