package fixture;

/** Atomic fixture: binary arithmetic, bits, shifts and negative constants. */
public final class ExprArith {
    public static int add(int left, int right) {
        return left + right;
    }

    public static int minusOne(int value) {
        return value - 1;
    }

    public static long multiply(long value) {
        return value * 17L;
    }

    public static int divide(int value, int divisor) {
        return value / divisor;
    }

    public static int modulo(int value) {
        return value % 7;
    }

    public static int bits(int value, int mask) {
        return (value & mask) | (value ^ mask);
    }

    public static int shifts(int value) {
        return (value << 2) + (value >> 1);
    }

    public static int negate(int value) {
        return -value;
    }
}
