package fixture;

/** Independent explanation; all native effects are modeled in README.md. */
final class CleanupFrontiers {
    static native FrontierHandle acquire(int mode, int ordinal);
    static native void log(Throwable failure);
    static native void probe();

    static int dual(int mode) throws Throwable {
        FrontierHandle first = null, second = null;
        int result = -1;
        try {
            try {
                first = acquire(mode, 1);
                second = acquire(mode, 2);
                if (first.ready()) result = first.read();
                if (second.ready()) {
                    try { result += second.read(); }
                    catch (RuntimeException recover) { log(recover); result++; }
                }
            } catch (java.io.InterruptedIOException stop) {
                throw stop;
            } catch (java.io.IOException recover) {
                log(recover);
                return -2;
            }
            return result;
        } finally {
            if (second != null) second.close();
            if (first != null) first.close();
        }
    }

    // The Smali shares the two identical catch bodies. probe is deliberately
    // outside BOTH intervals. One broad natural try would wrongly catch it.
    static int gap(int mode) throws Throwable {
        FrontierHandle handle = null;
        try { handle = acquire(mode, 1); }
        catch (java.io.IOException recover) {
            log(recover); if (handle != null) handle.close(); return -2;
        } catch (Throwable primary) {
            if (handle != null) handle.close(); throw primary;
        }
        probe();
        int result;
        try { result = handle.read(); }
        catch (java.io.IOException recover) {
            log(recover); if (handle != null) handle.close(); return -2;
        } catch (Throwable primary) {
            if (handle != null) handle.close(); throw primary;
        }
        if (handle != null) handle.close();
        return result;
    }
}

final class FrontierHandle {
    native boolean ready();
    native int read();
    native void close();
}
