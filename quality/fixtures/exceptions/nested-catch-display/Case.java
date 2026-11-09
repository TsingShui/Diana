package fixture;

/** Independent explanation; native effects are modeled in README.md. */
final class LayeredCatch {
    static native int step(int row, int mode);
    static native int recover(Throwable error, int row, int mode);
    static native void finish(int mode);

    // Inner and outer handlers share the same type. Lexical nesting must make
    // the inner catch win for the inner call.
    static int layered(int mode) {
        try {
            try { return step(0, mode); }
            catch (IllegalStateException inner) { return recover(inner, 0, mode); }
            finally { finish(mode); }
        } catch (IllegalStateException outer) { return recover(outer, 9, mode); }
    }

    // The second call is protected only by the inner handler of this catch body.
    static int catchBody(int mode) {
        try { return step(0, mode); }
        catch (IllegalStateException first) {
            try { return recover(first, 0, mode) + step(1, mode); }
            catch (IllegalArgumentException second) { return -2; }
        }
    }

    // Both adjacent typed clauses share one handler body.
    static int aliased(int mode) {
        try { return step(0, mode); }
        catch (IllegalStateException same) { return recover(same, 0, mode); }
        catch (IllegalArgumentException same) { return recover(same, 0, mode); }
    }
}
