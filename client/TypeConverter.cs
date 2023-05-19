public static class TypeConverter
{
	public record TypeConverters(Func<byte[], int, object> ToObject, Func<object, byte[]> GetBytes);

	public static byte GetTypeSize(Type type)
	{
		return (byte) TypeConvertersForType[type].GetBytes(Activator.CreateInstance(type)!).Length;
	}

	public static IReadOnlyDictionary<Type, TypeConverters> TypeConvertersForType = new[] {
			typeof(bool), typeof(byte), typeof(UInt16), typeof(UInt32), typeof(UInt64),
			typeof(sbyte), typeof(Int16), typeof(Int32), typeof(Int64), typeof(double)
		}
		.Select(type =>
			(type, typeMethods: new TypeConverters(ToObject: GetToObjectMethod(type), GetBytes: GetGetBytesMethod(type))))
		.ToDictionary(item => item.type, item => item.typeMethods);

	static Func<byte[], int, object> GetToObjectMethod(Type type) 
	{
		if (type == typeof(byte))
		{
			return (byte[] bytes, int startIndex) => bytes[startIndex];
		}

		if (type == typeof(sbyte))
		{
			return (byte[] bytes, int startIndex) => Convert.ToSByte(bytes[startIndex]);
		}

		var method = typeof(BitConverter)
			.GetMethods()
			.Where(m => m.GetParameters().Length == 2 &&
				m.Name.StartsWith("To") &&
				m.GetParameters()[0].ParameterType == typeof(byte[]) &&
				m.GetParameters()[1].ParameterType == typeof(int) &&
				m.ReturnType == type)
			.First();

		return (byte[] bytes, int startIndex) => method
			.Invoke(null, new object[] { bytes, startIndex })!;
	}

	static Func<object, byte[]> GetGetBytesMethod(Type type)
	{
		if (type == typeof(byte)) {
			return (object obj) => new byte[] { (byte) obj };
		}

		if (type == typeof(sbyte)) {
			return (object obj) => new byte[] { unchecked(((byte) (sbyte) obj)) };
		}

		var found = typeof(BitConverter)
			.GetMethods()
			.Where(method =>
				method.Name == "GetBytes" &&
				method.GetParameters().Length == 1
				&& method.GetParameters()[0].ParameterType == type)!;

		return (object obj) => (byte[]) found.First().Invoke(null, new[] { obj })!;
	}
}
