public abstract class IndexCommand : Command
{
	protected int index;

    protected override void ParseTokens()
    {
		if (tokens.Length > 2)
		{
			throw new Exception(
				$"Too many arguments! Required 0..1, but received {tokens.Length - 1}");
		}

		if (tokens.Length == 2)
		{
			index = int.Parse(tokens[1]);
			return;
		}

		index = state.GetLatestIndex();
    }

	protected void SetIndex()
	{
		bytes[1] = (byte) index;
	}

    protected override void PrepareRequestMessage()
    {
		bytes = new byte[2];

		SetCommand();
		SetIndex();
    }
}
