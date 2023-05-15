using System.Text;
using System.Net.Sockets;

public abstract class Command
{
	protected byte encoding;
	protected string[] tokens = null!;
	protected byte[] bytes = null!;
	protected State state = State.Instance;
	protected Socket socket = State.Instance.Socket;
	protected int receivedCount;

	private static IReadOnlyDictionary<string, Command> commands =
		Constants
		.CommandsByString
		.ToDictionary(item => item.Key, 
					  item => (Command) item.Value.GetProperty("Instance")!.GetValue(null)!);

	public static void Run(string[] tokens)
	{
		var command = commands[tokens[0]];
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
		var sentCount = state.Socket.Send(bytes);
		if (sentCount != bytes.Length)
		{
			throw new Exception(
				$"Sent only {sentCount} bytes out of {bytes.Length}");
		}
    }

    protected virtual void ReceiveResponseMessage()
    {
		receivedCount = state.Socket.Receive(bytes);
		if (receivedCount == 0)
		{
			throw new Exception("Received and empty packet");
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
