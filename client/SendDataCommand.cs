public sealed class SendDataCommand : Command
{
	bool isFilenameProvided = false;
	string filename = null!;

	Matrix matrix = null!;

	static readonly SendDataCommand instance = new SendDataCommand();
	public static SendDataCommand Instance { get => instance; }

	static SendDataCommand() {}
	private SendDataCommand()
	{
		encoding = Constants.CommandEncodings[GetType()];
	}

	protected override void ParseTokens()
	{
        if (tokens.Length > 2)
        {
			throw new Exception(
				$"Too many arguments! Required 0..1, but received {tokens.Length - 1}");
        }

		if (tokens.Length == 2)
		{
			filename = tokens[1];
			isFilenameProvided = true;
		}
	}

	private void ReadMatrix()
	{
		matrix = isFilenameProvided ? Matrix.FromFile(filename) : Matrix.FromCli();
	}

	private void SetMatrixType()
	{
		bytes[1] = Constants.TypeEncodings[matrix.Type];
	}

	private void SetMatrixDimension()
	{
		BitConverter.GetBytes(matrix.Dimension).CopyTo(bytes, 2);
	}

	private void SetMatrix()
	{
		matrix.Bytes.CopyTo(bytes, 6);
	}

	protected override void PrepareRequestMessage()
	{
		ReadMatrix();

		bytes = new byte[1 + 1 + 4 + matrix.Bytes.Length];

		SetCommand();
		SetMatrixType();
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
		state.AddMatrix(index, matrix);
    }
}