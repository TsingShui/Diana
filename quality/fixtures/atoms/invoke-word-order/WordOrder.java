package fixture;

import java.lang.invoke.MethodHandle;

public final class WordOrder {
    public static native int take(long value, int tail);
    public static native int take(double first, long value, int tail);
    public static native int finish(int left, int right, int[] elements);

    public static int run(double first, long value, int tail, MethodHandle handle) throws Throwable {
        int left = take(value, tail);
        int middle = take(first, value, tail);
        int right = (int) handle.invokeExact(value);
        return finish(left, right, new int[] { left, middle, right });
    }
}
