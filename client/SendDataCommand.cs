public sealed class SendDataCommand : Command
{
	string? filename = null!;

	Matrix matrix = null!;

	static SendDataCommand? instance;
	public static SendDataCommand GetInstance() 
	{
		if (instance == null)
		{
			instance = new SendDataCommand();
		}

		return instance;
	}

	static SendDataCommand() {}
	private SendDataCommand()
	{
		encoding = Constants.Instance.GetCommandEncodings()[GetType()];
	}

	protected override void ParseTokens()
	{
		filename = null;

        if (tokens.Length > 2)
        {
			throw new Exception("The command takes no more than one argument!");
        }

		if (tokens.Length == 2)
		{
			filename = tokens[1];
		}
	}

	private void ReadMatrix()
	{
		matrix = filename != null ? Matrix.FromFile(filename) : Matrix.FromCli();
	}

	private void SetMatrixTypeSize()
	{
		bytes[1] = matrix.TypeSize;
	}

	private void SetMatrixDimension()
	{
		BitConverter.GetBytes(matrix.Dimension).CopyTo(bytes, 2);
	}

	private void SetMatrix()
	{
		matrix.Bytes.CopyTo(bytes.AsSpan(6));
	}

	protected override void PrepareRequestMessage()
	{
		ReadMatrix();

		bufferSize = 1 + 1 + 4 + matrix.Bytes.Length;
		if (bytes.Length < bufferSize)
		{
			bytes = new byte[bufferSize];
		}

		SetCommand();
		SetMatrixTypeSize();
		SetMatrixDimension();
		SetMatrix();
	}

	private byte GetIndex()
	{
		return bytes[1];
	}

    protected override void HandleResponseMessage()
    {
		var success = GetSuccessCode();
		if (!success)
		{
			throw new Exception(GetError());
		}

		var index = GetIndex();
		state.SendDataSet(index, matrix);

		Console.WriteLine($"Your job was registered at index {index}.");
    }
}