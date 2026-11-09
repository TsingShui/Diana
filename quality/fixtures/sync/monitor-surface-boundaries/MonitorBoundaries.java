package fixture;
public final class MonitorBoundaries {
    public static native int read(int stage, int mode);
    public static native void note(Throwable error);
    public static int nested(Object outer, Object inner, int mode) {
        int total;
        synchronized (outer) {
            int a = read(0, mode), b;
            synchronized (inner) { b = read(1, mode); }
            int c = read(2, mode); total = a + b + c;
        }
        return total;
    }
    public static int innerCatch(Object lock, int mode) {
        int value;
        synchronized (lock) {
            try { value = read(0, mode); }
            catch (IllegalStateException error) { value = 10; }
        }
        return value;
    }
    // acquire/release below explain original monitor-enter/exit only; these
    // are not calls in the Smali. DEX-only leaking or split outlets are deliberate.
    private static native void acquire(Object lock);
    private static native void release(Object lock);
    public static int extra(Object lock, int mode) throws Throwable {
        acquire(lock);
        int value;
        try { value = read(0, mode); }
        catch (Throwable error) { note(error); release(lock); throw error; }
        release(lock); return value;
    }
    public static int otherException(Object lock, Throwable other, int mode) throws Throwable {
        acquire(lock);
        int value;
        try { value = read(0, mode); }
        catch (Throwable error) { release(lock); throw other; }
        release(lock); return value;
    }
    public static int twoReturns(Object lock, int mode) throws Throwable {
        acquire(lock); int value;
        try { value = read(0, mode); }
        catch (Throwable error) { release(lock); throw error; }
        if (mode != 0) { value += 100; release(lock); return value; }
        release(lock); return value;
    }
    public static int wrongLock(Object lock, Object other, int mode) throws Throwable {
        acquire(lock); int value;
        try { value = read(0, mode); }
        catch (Throwable error) { release(other); throw error; }
        release(lock); return value;
    }
    public static Throwable escaped(int mode) {
        Throwable result;
        try { read(0, mode); result = null; }
        catch (Throwable error) { result = error; }
        return result;
    }
}
