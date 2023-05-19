using System.Text;
using System.Net.Sockets;

public abstract class Command
{
	protected byte encoding;
	protected string[] tokens = null!;
	protected static byte[] bytes = new byte[1500];
	protected State state = State.GetInstance();
	protected Socket socket = State.GetInstance().Socket;
	protected int receivedCount;
	protected int bufferSize;

	public static void Run(string[] tokens)
	{
		var command = Constants.Instance.GetCommandsByString()[tokens[0]];
		command.tokens = tokens;
		command.Run();
	}

	protected void SetCommand()
	{
		bytes[0] = encoding;
	}

	protected bool GetSuccessCode()
	{
		if (bytes[0] > 1)
		{
			throw new Exception(
				$"Unknown server response: bytes[0] is {bytes[0]}");
		}

		return bytes[0] == 0;
	}

	protected string GetError()
	{
		return Encoding.UTF8.GetString(bytes, 1, receivedCount - 1);
	}

	protected void Run()
	{
		ParseTokens();
		PrepareRequestMessage();
		SendRequestMessage();
		ReceiveResponseMessage();
		HandleResponseMessage();
	}

	protected abstract void ParseTokens();
	protected abstract void PrepareRequestMessage();

    protected virtual void SendRequestMessage()
    {
		var sentCount = state.Socket.Send(bytes, 0, bufferSize, SocketFlags.None);
		if (sentCount != bufferSize)
		{
			throw new Exception(
				$"Sent only {sentCount} bytes out of {bufferSize}");
		}
    }

    protected virtual void ReceiveResponseMessage()
    {
		bufferSize = 1500;
		receivedCount = state.Socket.Receive(bytes, 0, bufferSize, SocketFlags.None);
		if (receivedCount == 0)
		{
			throw new Exception("The server disconnected");
		}
    }

	protected virtual void HandleResponseMessage()
	{
		var success = GetSuccessCode();
		if (!success)
		{
			throw new Exception(GetError());
		}
	}
}
