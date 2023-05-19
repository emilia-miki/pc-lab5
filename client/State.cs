using System.Net.Sockets;

public sealed class State
{
    static State? _instance;
    public static State GetInstance()
    {
        if (_instance == null)
        {
            _instance = new State();
        }

        return _instance;
    }

    static State() {}
    State() {}

    public Socket Socket { get; set; } = null!;

    public record MatrixSpec(byte TypeSize, Type Type, UInt32 Dimension, int BufferLength);

    Queue<byte> indicesToCalculate = new();
    Dictionary<int, MatrixSpec> matrixSpecs = new();

    Queue<byte> indicesCalculating = new();

    public void SendDataSet(byte index, Matrix matrix)
    {
        indicesToCalculate.Enqueue(index);
        matrixSpecs.Add(index, new MatrixSpec(matrix.TypeSize, matrix.Type, matrix.Dimension, matrix.Bytes.Length));
    }

    public MatrixSpec GetMatrixSpecs(byte index)
    {
        if (!matrixSpecs.ContainsKey(index))
        {
            throw new Exception($"There was no matrix registered for index {index}");
        }

        return matrixSpecs[index];
    }

    public byte StartCalculationGet()
    {
        if (indicesToCalculate.Count == 0)
        {
            throw new Exception("You have not sent any matrices to transpose");
        }

        return indicesToCalculate.Dequeue();
    }

    public void StartCalculationSet(byte index)
    {
        indicesCalculating.Enqueue(index);
    }

    public byte GetStatusGet()
    {
        if (indicesCalculating.Count == 0)
        {
            throw new Exception("There are no running jobs to check the status of!");
        }

        return indicesCalculating.Dequeue();
    }

    public void GetStatusSet(byte index)
    {
        indicesCalculating.Enqueue(index);
    }
}
