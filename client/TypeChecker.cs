using System.Globalization;

public static class TypeChecker
{
    // Short for Least Common Type
    enum LCT
    {
        Bool, Unsigned, Signed, Double
    }

    enum USize
    {
        U8, U16, U32, U64
    }

    enum SSize
    {
        S8, S16, S32, S64
    }

    static void CheckUSize(decimal num, ref USize uSize)
    {
    Switch: switch (uSize)
        {
            case USize.U8:
                if (byte.MaxValue < num)
                {
                    uSize = USize.U16;
                    goto Switch;
                }
                break;
            case USize.U16:
                if (UInt16.MaxValue < num)
                {
                    uSize = USize.U32;
                    goto Switch;
                }
                break;
            case USize.U32:
                if (UInt32.MaxValue < num)
                {
                    uSize = USize.U64;
                    goto Switch;
                }
                break;
            case USize.U64:
                if (UInt64.MaxValue < num)
                {
                    throw new Exception("The element size is too large for supported types");
                }
                break;
        }
    }

    static void CheckSSize(decimal num, ref SSize sSize)
    {
    Switch: switch (sSize)
        {
            case SSize.S8:
                if (num < sbyte.MinValue || sbyte.MaxValue < num)
                {
                    sSize = SSize.S16;
                    goto Switch;
                }
                break;
            case SSize.S16:
                if (num < Int16.MinValue || Int16.MaxValue < num)
                {
                    sSize = SSize.S32;
                    goto Switch;
                }
                break;
            case SSize.S32:
                if (num < Int32.MinValue || Int32.MaxValue < num)
                {
                    sSize = SSize.S64;
                    goto Switch;
                }
                break;
            case SSize.S64:
                if (num < Int64.MinValue || Int64.MaxValue < num)
                {
                    throw new Exception("The element size is too large for supported types");
                }
                break;
        }
    }

    public static Type DetermineType(string[] stringArray)
    {
        var lct = LCT.Bool;
        var uSize = USize.U8;
        var sSize = SSize.S8;
        foreach (var str in stringArray)
        {
            decimal res = 0;

        SwitchParser: switch (lct)
            {
                case LCT.Bool:
                case LCT.Unsigned:
                case LCT.Signed:
                    if (!decimal.TryParse(str, NumberStyles.Any, CultureInfo.InvariantCulture.NumberFormat, out res))
                    {
                        lct = LCT.Double;
                        goto SwitchParser;
                    }
                    break;
                case LCT.Double:
                    if (!double.TryParse(str, NumberStyles.Any, CultureInfo.InvariantCulture.NumberFormat, out var _))
                    {
                        throw new Exception("The matrix elements do not match any of the supported types");
                    }
                    break;
            }

        SwitchLCT: switch (lct)
            {
                case LCT.Bool:
                    if (res != 0 || res != 1)
                    {
                        lct = LCT.Unsigned;
                        goto SwitchLCT;
                    }
                    break;
                case LCT.Unsigned:
                    if (res < 0)
                    {
                        lct = LCT.Signed;
                        goto SwitchLCT;
                    }

                    CheckUSize(res, ref uSize);
                    break;
                case LCT.Signed:
                    if (Decimal.Remainder(res, 1) != 0)
                    {
                        lct = LCT.Double;
                    }

                    CheckSSize(res, ref sSize);
                    break;
            }
        }

        return (lct, uSize, sSize) switch
        {
            (LCT.Bool, _, _) => typeof(bool),
            (LCT.Unsigned, USize.U8, _) => typeof(byte),
            (LCT.Unsigned, USize.U16, _) => typeof(UInt16),
            (LCT.Unsigned, USize.U32, _) => typeof(UInt32),
            (LCT.Unsigned, USize.U64, _) => typeof(UInt64),
            (LCT.Signed, _, SSize.S8) => typeof(sbyte),
            (LCT.Signed, _, SSize.S16) => typeof(Int16),
            (LCT.Signed, _, SSize.S32) => typeof(Int32),
            (LCT.Signed, _, SSize.S64) => typeof(Int64),
            (LCT.Double, _, _) => typeof(double),
            _ => throw new Exception("Inconsistend internal state"),
        };
    }
}
