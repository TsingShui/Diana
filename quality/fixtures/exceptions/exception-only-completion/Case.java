package fixture;

/** Independent control explanation; native effects are modeled in README.md. */
final class ExceptionCompletion {
    static native void touch(int mode);
    static native void outside(int mode);
    static native int consume(Throwable failure, int mode);
    static native void finish(int mode);

    static int typed(int mode, Throwable failure) throws Throwable {
        try { touch(mode); throw failure; }
        catch (IllegalStateException caught) { return consume(caught, mode); }
    }

    static int nested(int mode, Throwable failure) throws Throwable {
        try {
            try { touch(mode); throw failure; }
            catch (IllegalStateException caught) { return consume(caught, mode); }
        } finally { finish(mode); }
    }

    // Smali has one shared throw target. The outside call is unprotected and
    // enters that target without traversing the protected touch entry.
    static int foreign(int mode, Throwable failure) throws Throwable {
        if (mode != 0) outside(mode);
        try {
            if (mode == 0) touch(mode);
            throw failure;
        } catch (IllegalStateException caught) { return consume(caught, mode); }
    }
}
