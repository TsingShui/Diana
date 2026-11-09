package fixture;

/** Independent explanation; native behavior is modeled in README.md. */
final class ReadableCatch {
    static native int step(int row, int mode);
    static native int recover(Throwable error, int row, int mode);
    static native void outside(int mode);

    static int run(int limit, int mode) {
        int row = 0, total = 0;
        while (row < limit) {
            try { total += step(row, mode); }
            catch (IllegalStateException error) { total += recover(error, row, mode); }
            row++;
        }
        return total;
    }
    static int rethrow(int mode, Throwable error) throws Throwable {
        try { step(0, mode); throw error; }
        catch (IllegalStateException caught) { return recover(caught, 0, mode); }
    }
    // Smali shares a handler, with an unprotected outside call between intervals.
    static int foreign(int mode) {
        int first;
        try { first = step(0, mode); }
        catch (IllegalStateException error) { return recover(error, 0, mode); }
        outside(mode);
        try { return first + step(1, mode); }
        catch (IllegalStateException error) { return recover(error, 1, mode); }
    }
    static int multi(int mode) {
        try { return step(0, mode); }
        catch (IllegalStateException error) { return recover(error, 0, mode); }
        catch (IllegalArgumentException other) { return -2; }
    }
}
