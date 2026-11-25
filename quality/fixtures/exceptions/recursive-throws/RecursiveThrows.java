package fixture;

public final class RecursiveThrows {
    public static void a(boolean recurse) throws Throwable {
        b(recurse);
    }

    public static void b(boolean recurse) throws Throwable {
        try { c(recurse); } catch (ReadFailure handled) { }
    }

    public static void c(boolean recurse) throws Throwable {
        if (recurse) b(false);
        else zRead();
    }

    public static void d() throws Throwable {
        try { c(false); } catch (ReadFailure handled) { }
        try { c(false); } catch (Throwable handled) { }
    }

    public static void e() {
        try { d(); } catch (Throwable handled) { }
    }

    public static native void zRead() throws Throwable;
}

final class ReadFailure extends Throwable { }
