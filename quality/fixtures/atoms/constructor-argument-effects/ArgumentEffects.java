package fixture;

public final class ArgumentEffects {
    public static native int read(int stage, int fault);
    public static native int finish(int value);
    public static native String message(int value);
    public static native void note(int value, Throwable error);
    public static Point tree(int base, int fault) {
        try { return new Point(finish(read(0, fault) + read(1, fault)), read(2, fault) + base); }
        catch (Throwable error) { note(base, error); return null; }
    }
    public static void caughtError(int fault) {
        try { throw new IllegalArgumentException(message(read(0, fault))); }
        catch (Throwable error) { note(0, error); }
    }
    // The following guards expose DEX-only allocation-before-temporaries order.
    // allocatePoint is an explanatory new-instance operation, not an assembly call.
    private static native Point allocatePoint();
    private static native void initializePoint(Point point, int x, int y);
    public static Point reversed(int fault) {
        Point point = allocatePoint();
        int y = read(1, fault), x = read(0, fault);
        initializePoint(point, x, y); return point;
    }
    public static Point shared(int fault) {
        Point point = allocatePoint(); int value = read(0, fault);
        initializePoint(point, value, value); return point;
    }
    public static Point interleaved(int fault) {
        Point point = allocatePoint();
        int x = read(0, fault), y = read(1, fault); x = finish(x);
        initializePoint(point, x, y); return point;
    }
    public static int changingSnapshot(int fault) {
        int saved = 0;
        try { Point point = allocatePoint(); saved = 1;
            initializePoint(point, read(0, fault), saved); return saved; }
        catch (Throwable error) { note(saved, error); return saved; }
    }
    // differentProtection's per-operation handlers are recorded in Smali/README.
}
final class Point {
    public int x, y;
    public Point(int x, int y) { this.x = x; this.y = y; }
}
