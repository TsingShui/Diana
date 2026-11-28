public final class NaNChild extends ObservingParent {
    public static final float F_NAN = Float.intBitsToFloat(0x7fc01234);
    public static final double D_NAN = Double.longBitsToDouble(0xfff8000000005678L);
    public static final float F_ZERO = -0.0f;
    public static final double D_INF = Double.POSITIVE_INFINITY;

    public static int activate() {
        return 3;
    }
}
