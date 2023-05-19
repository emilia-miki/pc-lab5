using System.Text;
using System.Globalization;

public class Matrix
{
	public UInt32 Dimension { get; }
	public Type Type { get; }
	public byte TypeSize { get; }
	public ReadOnlySpan<byte> Bytes { get => bytes; }
	byte[] bytes;

	Matrix(byte typeSize, Type type, UInt32 dimension, byte[] bytes)
	{
		TypeSize = typeSize;
		Type = type;
		Dimension = dimension;
		this.bytes = bytes;
	}

	public static Matrix FromBytes(byte typeSize, Type type, UInt32 dimension, byte[] bytes)
	{
		return new Matrix(typeSize, type, dimension, bytes);
	}

	public static Matrix FromFile(string filename)
	{
		Console.WriteLine("Loading the matrix...");

		string[] matrixStringArray;
		try
		{
	        matrixStringArray =
				File
				.ReadLines(filename)
				.Where(line => !string.IsNullOrWhiteSpace(line))
				.SelectMany(line => line.Split(','))
				.Select(line => line.Trim())
				.ToArray();
			
		}
		catch (FileNotFoundException)
		{
			throw new Exception("File not found");
		}

		var matrix = Matrix.FromStringArray(matrixStringArray);

		Console.WriteLine("The matrix loaded successfully");

		return matrix;
	}

	public static Matrix FromCli()
	{
        var lines = new List<string>();

		string? line = null;
        while (string.IsNullOrWhiteSpace(line))
        {
	        line = Console.ReadLine();
        }

        lines.Add(line);

        var dimension = (UInt32) line.Split(' ').Length;
        for (var i = 0; i < dimension - 1; i++)
		while (lines.Count() < dimension)
        {
            line = Console.ReadLine();
			if (string.IsNullOrWhiteSpace(line))
			{
				continue;
			}

            lines.Add(line);
        }

        var matrixStringArray = lines.SelectMany(line => line.Split(' ')).Select(line => line.Trim()).ToArray();
		return Matrix.FromStringArray(matrixStringArray);
	}

	public void ToFile(string filename)
	{
		using (var file = File.Open(filename, FileMode.Create))
		{
			using (var writer = new StreamWriter(file))
			{
				var builder = new StringBuilder();
				var i = 0;
				foreach (var value in AsTypeEnumerable())
				{
					var str = Convert.ToString(value, CultureInfo.InvariantCulture.NumberFormat);
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

					i += 1;
				}

				writer.Write(builder);
			}
		}
	}

	public void ToCli()
	{
		var i = 0;
		foreach (var value in AsTypeEnumerable())
		{
			var str = Convert.ToString(value, CultureInfo.InvariantCulture.NumberFormat);
			Console.Write(str);

			if ((i + 1) % Dimension == 0)
			{
				Console.WriteLine();
			}
			else
			{
				Console.Write(" ");
			}

			i += 1;
		}
	}

	IEnumerable<object> AsTypeEnumerable()
	{
		return bytes
			.Select((b, index) => new { b, index })
			.GroupBy(x => x.index / TypeSize)
			.Select(g => TypeConverter.TypeConvertersForType[Type].ToObject(g.Select(g => g.b).ToArray(), 0));
	}

	static Matrix FromStringArray(string[] stringArray)
	{
		var sqrt = Math.Sqrt(stringArray.Length);
		var dimension = (uint) sqrt;

		if (sqrt != dimension)
		{
			throw new Exception($"This is not a square matrix (element count is {stringArray.Length})");
		}

		var type = TypeChecker.DetermineType(stringArray);
		var typeSize = TypeConverter.GetTypeSize(type);
		var bytes = new byte[dimension * dimension * typeSize];

		for (var i = 0; i < stringArray.Length; i++)
		{
			var obj = Convert.ChangeType(stringArray[i], type, CultureInfo.InvariantCulture.NumberFormat);
			var strBytes = TypeConverter.TypeConvertersForType[type].GetBytes(obj);

			strBytes.CopyTo(bytes, i * typeSize);
		}

		return new Matrix(TypeConverter.GetTypeSize(type), type, dimension, bytes);
	}
}
