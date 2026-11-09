package fixture;

public final class ConstructorArguments {
    public static native Resources getSystem();
    public static native void note(Throwable error);
    public static Point point() {
        return new Point(getSystem().getDisplayMetrics().widthPixels,
                         getSystem().getDisplayMetrics().heightPixels);
    }
    public static Point caughtPoint() {
        try {
            return new Point(getSystem().getDisplayMetrics().widthPixels,
                             getSystem().getDisplayMetrics().heightPixels);
        } catch (Throwable error) { note(error); return null; }
    }
    public static void bodyError(int mode) {
        try { if (mode != 0) throw new IllegalStateException("body"); }
        catch (Throwable error) { note(error); }
    }
}
final class Resources { public native Metrics getDisplayMetrics(); }
final class Metrics { public int widthPixels; public int heightPixels; }
final class Point {
    public int x, y;
    public Point(int x, int y) { this.x = x; this.y = y; }
}
