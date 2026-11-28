package fixture;

public final class FillData {
    public static int[] shared;
    public static int trace;
    public static ArithmeticException caught;

    public static void fillTwo(int[] target) {
        System.arraycopy(new int[] {11, 22}, 0, target, 0, 2);
    }

    public static void fillEmpty(int[] target) {
        if (target.length < 0) {
            throw new AssertionError();
        }
    }

    public static void fillFloat(float[] target) {
        System.arraycopy(new float[] {1.5f, -0.0f}, 0, target, 0, 2);
    }

    public static void fillFloatBits(float[] target) {
        System.arraycopy(new float[] {0.0f, -0.0f, Float.intBitsToFloat(0x7fc12345)}, 0, target, 0, 3);
    }

    public static int[] receiver() {
        trace++;
        return shared;
    }

    public static void fillFromReceiver() {
        fillTwo(receiver());
    }

    public static int[] receiverMaybe(int[] target, ArithmeticException fault) {
        trace++;
        if (fault != null) {
            throw fault;
        }
        return target;
    }

    public static int protectedFill(int[] target, ArithmeticException fault) {
        try {
            fillTwo(receiverMaybe(target, fault));
            return 1;
        } catch (ArrayIndexOutOfBoundsException bounds) {
            return target[0];
        } catch (ArithmeticException failure) {
            caught = failure;
            return -1;
        }
    }
}
