using System.Diagnostics;

public class Debug
{
	[Conditional("DEBUG")]
	public static void WriteLine(string str)
	{
		Console.WriteLine(str);
	}
}
