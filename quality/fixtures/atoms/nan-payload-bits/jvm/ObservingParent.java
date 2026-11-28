public class ObservingParent {
    static final int parentFloatBits;
    static final long parentDoubleBits;

    static {
        try {
            Class<?> child = Class.forName("NaNChild", false, ObservingParent.class.getClassLoader());
            parentFloatBits = Float.floatToRawIntBits(child.getDeclaredField("F_NAN").getFloat(null));
            parentDoubleBits = Double.doubleToRawLongBits(child.getDeclaredField("D_NAN").getDouble(null));
        } catch (ReflectiveOperationException error) {
            throw new AssertionError(error);
        }
    }
}
