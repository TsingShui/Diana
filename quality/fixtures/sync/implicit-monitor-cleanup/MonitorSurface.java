package fixture;
import java.util.ArrayDeque;
public final class MonitorSurface {
    public ArrayDeque<?> runningAsyncCalls;
    public ArrayDeque<?> runningSyncCalls;
    public int counts() {
        int sum;
        synchronized (this) {
            int async = runningAsyncCalls.size();
            int sync = runningSyncCalls.size();
            sum = async + sync;
        }
        return sum;
    }
    public static native int read(int mode);
    public static native void note(Throwable error);
    public static int caught(int mode) {
        try { return read(mode); }
        catch (Throwable error) { note(error); return -1; }
    }
}
