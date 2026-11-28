import java.util.Locale;

public final class CheckNaNPayloads {
    public static void main(String[] args) {
        f("ldc-float", Float.floatToRawIntBits(NaNWork.floatNaN()));
        d("ldc-double", Double.doubleToRawLongBits(NaNWork.doubleNaN()));
        f("select-float-true", NaNWork.selectFloatBits(true));
        f("select-float-false", NaNWork.selectFloatBits(false));
        d("select-double-true", NaNWork.selectDoubleBits(true));
        d("select-double-false", NaNWork.selectDoubleBits(false));
        f("negative-zero", Float.floatToRawIntBits(NaNWork.negativeZero()));
        d("positive-infinity", Double.doubleToRawLongBits(NaNWork.positiveInfinity()));
    }

    private static void f(String caseName, int bits) {
        System.out.printf(Locale.ROOT, "%s\t%08x%n", caseName, bits);
    }

    private static void d(String caseName, long bits) {
        System.out.printf(Locale.ROOT, "%s\t%016x%n", caseName, bits);
    }
}

final class CheckNaNPreparation {
    public static void main(String[] args) throws ReflectiveOperationException {
        // Active use of NaNChild causes the superclass to read its fields recursively.
        System.out.println("active\t" + NaNChild.activate());
        System.out.printf(Locale.ROOT, "parent\t%08x,%016x%n",
                ObservingParent.parentFloatBits, ObservingParent.parentDoubleBits);
        Class<?> type = NaNChild.class;
        System.out.printf(Locale.ROOT, "fields\t%08x,%016x%n",
                Float.floatToRawIntBits(type.getDeclaredField("F_NAN").getFloat(null)),
                Double.doubleToRawLongBits(type.getDeclaredField("D_NAN").getDouble(null)));
        System.out.printf(Locale.ROOT, "controls\t%08x,%016x%n",
                Float.floatToRawIntBits(type.getDeclaredField("F_ZERO").getFloat(null)),
                Double.doubleToRawLongBits(type.getDeclaredField("D_INF").getDouble(null)));
    }
}
