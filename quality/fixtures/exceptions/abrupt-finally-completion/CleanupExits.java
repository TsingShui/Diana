package fixture;

/** Independent explanation of the assembly; no Java compilation is required. */
public final class CleanupExits {
    public static int trace;
    public static void mark(int event) { trace = trace * 31 + event; }
    public static int read(int stage, int fault) {
        mark(stage + 10);
        if ((fault & (1 << stage)) != 0) throw new IllegalStateException("read");
        return stage + 7;
    }
    public static void close(int stage, int fault) {
        mark(stage + 30);
        if ((fault & (16 << stage)) != 0) throw new IllegalArgumentException("close");
    }
    public static int capture(int mode, int base, int fault) throws Throwable {
        int saved;
        try { saved = read(0, fault) + base; }
        finally {
            base += 100;
            close(0, fault);
            mark(base);
            if ((mode & 2) != 0) return -7;
            if ((mode & 4) != 0) throw new IllegalArgumentException("override");
        }
        return saved;
    }
    public static int always(int mode, int fault) throws Throwable {
        try { read(0, fault); }
        finally {
            close(0, fault);
            if ((mode & 4) != 0) throw new IllegalArgumentException("override");
            return 8;
        }
    }
    public static int twoRethrows(int mode, int fault) throws Throwable {
        try { read(0, fault); }
        finally {
            if ((mode & 1) != 0) close(0, fault);
            else close(1, fault);
        }
        return 8;
    }
    public static void voidAlways(int fault) throws Throwable {
        try { read(0, fault); }
        finally { close(0, fault); return; }
    }
    public static void bareVoid(int fault) {
        try { read(0, fault); }
        finally { return; }
    }
    public static void parameterThrow(Throwable error, int fault) throws Throwable {
        try { read(0, fault); }
        finally { throw error; }
    }
    public static int differentReturn(int fault) throws Throwable {
        try { read(0, fault); }
        catch (Throwable failure) { close(0, fault); return -8; }
        close(0, fault);
        return -7;
    }
    public static void differentThrow(int fault) throws Throwable {
        try { read(0, fault); }
        catch (Throwable failure) { close(0, fault); throw new IllegalArgumentException("handler"); }
        close(0, fault);
        throw new IllegalArgumentException("normal");
    }
    public static int foreign(int mode, int fault) throws Throwable {
        if ((mode & 8) == 0) {
            try { read(0, fault); }
            catch (Throwable failure) { close(0, fault); return 8; }
        }
        close(0, fault);
        return 8;
    }
}
