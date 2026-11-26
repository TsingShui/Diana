package fixture;

public final class BoundValues {
    public static native boolean take(float x, float y);
    public static native void done(float x, float y);
    public static native void keep(int x, int y);
    public static native void mayThrow();
    public static native void replacement(Throwable value);

    public static int guarded(int x, boolean reset) {
        if (reset) {
            x = 0;
        }
        if (x == 0) {
            return -1;
        }
        float f = (float) x;
        if (!take(f, f)) {
            done(f, f);
        }
        return x;
    }

    public static int pair(int x, int y, boolean resetX, boolean resetY) {
        if (resetX) {
            x = 0;
        }
        if (resetY) {
            y = 0;
        }
        if (x == 0 && y == 0) {
            return -1;
        }
        float first = (float) x;
        float second = (float) y;
        if (!take(first, second)) {
            done(first, second);
            keep(x, y);
        }
        return x + y;
    }

    public static int caughtDefault(boolean clear, boolean repeat) {
        try {
            mayThrow();
            return -1;
        } catch (RuntimeException exception) {
            if (clear) {
                exception = null;
            }
            if (repeat) {
                replacement(exception);
            }
            int selected = exception instanceof IllegalStateException ? 1 : 0;
            if (repeat) {
                keep(selected, 5);
            }
            keep(selected, 5);
            replacement(exception);
            return selected;
        }
    }

    public static void localConstant(boolean repeat) {
        float value = (float) 37;
        if (repeat) {
            done(value, value);
        }
        done(value, value);
    }
}
