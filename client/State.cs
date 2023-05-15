using System.Net.Sockets;

public sealed class State
{
    static readonly State _instance = new State();
    public static State Instance { get => _instance; }

    static State() {}
    State() {}

    public Socket Socket { get; set; } = null!;

    byte latestIndex;
    bool isLatestIndexSet = false;

    public byte GetLatestIndex() =>
            isLatestIndexSet 
            ? latestIndex
            : throw new Exception("No matrices have been sent yet");

    Dictionary<int, Matrix> _matrices = new();

    public void AddMatrix(byte index, Matrix matrix) 
    {
        _matrices.Add(index, matrix);  
        latestIndex = index;
        isLatestIndexSet = true;
    }

    public Matrix GetMatrix(int index) => _matrices[index];
}
