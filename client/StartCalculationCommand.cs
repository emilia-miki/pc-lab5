public class StartCalculationCommand : IndexCommand
{
	private readonly StartCalculationCommand instance =
		new StartCalculationCommand();
	public StartCalculationCommand Instance { get => instance; }

	static StartCalculationCommand() {}
	private StartCalculationCommand()
	{
		encoding = Constants.CommandEncodings[GetType()];
	}

    protected override void HandleResponseMessage()
    {
		base.HandleResponseMessage();

		Console.WriteLine("Calculation started!");
    }
}
