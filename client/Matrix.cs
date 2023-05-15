using System.Reflection;
using System.Text;

public class Matrix
{
	public UInt32 Dimension { get; }
	public Type Type { get; }
	public byte TypeSize { get; }

	public byte[] Bytes { get => bytes; }

	bool isArrayInitialized = false;
	Array array = null!;
	byte[] bytes = null!;

	private MethodInfo toObjectMethod;
	private MethodInfo getBytesMethod;

	public Array GetArray()
	{
		if (!isArrayInitialized)
		{
			ParseBytes();
			isArrayInitialized = true;
		}

		return array;
	}

	public class InvalidTypeException : Exception {}

	private static byte GetTypeSize(Type type) =>
		BitConverter.GetBytes((dynamic) Activator.CreateInstance(type)!).Length;

	private Matrix(UInt32 dimension, Type type)
	{
		Dimension = dimension;
		Type = type;
		TypeSize = Matrix.GetTypeSize(type);

		toObjectMethod = typeof(BitConverter).GetMethod($"To{type}",
			new[] { typeof(byte[]), typeof(int) })!;
		getBytesMethod = typeof(BitConverter).GetMethod("GetBytes", new[] { type })!;

		Debug.WriteLine(
			$"Created an instance of MatrixObject. N = {dimension}, type = {type}, " +
			$"element size = {TypeSize} bytes.");
	}

	private object BytesToObject(int itemIndex)
	{
		var startIndex = itemIndex * TypeSize;

		// TODO: why can't i do this without copying
		return toObjectMethod.Invoke(null, new object[] { bytes, startIndex })!;
	}

	private byte[] StrToBytes(string str)
	{
		var value = Convert.ChangeType(str, Type);

		// TODO: why can't i do this without copying
		return (byte[]) getBytesMethod.Invoke(null, new[] { value })!;
	}

	private void ParseLines(string[] lines)
	{
		bytes = new byte[TypeSize * lines.Length * lines.Length];

		for (var i = 0; i < lines.Count(); i++)
		{
			StrToBytes(lines[i]).CopyTo(bytes, i * TypeSize);
		}
	}

	private void ParseBytes()
	{
		if (bytes.Length != TypeSize * Dimension * Dimension)
		{
			throw new InvalidTypeException();
		}

		var arrayLength = Dimension * Dimension;
		array = Array.CreateInstance(Type, arrayLength);
		for (var i = 0; i < arrayLength; i++)
		{
			array.SetValue(BytesToObject(i), i);
		}
	}

	public static Matrix FromLines(string[] lines)
	{
		var sqrt = Math.Sqrt(lines.Length);
		var dimension = (UInt32) sqrt;
		if (sqrt != dimension)
		{
			throw new InvalidTypeException();
		}

		foreach (var type in Constants.Types)
		{
			var matrix = new Matrix(dimension, type);

			try
			{
				matrix.ParseLines(lines);
			}
			catch
			{
				continue;
			}

			return matrix;
		}

		throw new InvalidTypeException();
	}

	public static Matrix FromBytes(byte[] bytes, Type type, UInt32 dimension)
	{
		var matrix = new Matrix(dimension, type);

		matrix.bytes = bytes;
		matrix.ParseBytes();

		return matrix;
	}

	public static Matrix FromFile(string filename)
	{
        var lines =
			File
			.ReadLines(filename)
			.Where(line => !string.IsNullOrWhiteSpace(line))
			.SelectMany(line => line.Split(','))
			.ToArray();

		return Matrix.FromLines(lines);
	}

	public static Matrix FromCli()
	{
        var lines = new List<string>();
        var line = Console.ReadLine();
        if (string.IsNullOrWhiteSpace(line))
        {
			throw new Exception("Could not read line");
        }

        lines.Add(line);

        var dimension = (UInt32) line.Split(' ').Length;
        for (var i = 0; i < dimension - 1; i++)
        {
            line = Console.ReadLine();
			if (string.IsNullOrWhiteSpace(line))
			{
				throw new Exception("Could not read line");
			}

            lines.Add(line);
        }

        var matrixLines = lines.SelectMany(line => line.Split(',')).ToArray();
		return Matrix.FromLines(matrixLines);
	}

	public void ToFile(string filename)
	{
		using (var file = File.Open(filename, FileMode.Create))
		{
			using (var writer = new StreamWriter(file))
			{
				var arr = GetArray();

				var builder = new StringBuilder();
				for (var i = 0; i < arr.Length; i++)
				{
					var value = arr.GetValue(i);
					var str = Convert.ToString(value);
					builder.Append(str);

					if ((i + 1) % Dimension == 0)
					{
						builder.AppendLine();
						writer.Write(builder);
						builder.Clear();
					}
					else
					{
						builder.Append(", ");
					}
				}

				writer.Write(builder);
			}
		}
	}

	public void ToCli()
	{
		var arr = GetArray();
		for (var i = 0; i < arr.Length; i++)
		{
			var value = arr.GetValue(i);
			var str = Convert.ToString(value);
			Console.Write(str);

			if ((i + 1) % Dimension == 0)
			{
				Console.WriteLine();
			}
			else
			{
				Console.Write(" ");
			}
		}
	}
}
