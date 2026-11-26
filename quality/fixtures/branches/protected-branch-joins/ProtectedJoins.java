package fixture;

public final class ProtectedJoins {
    public static native int before(int value);
    public static native int left(int value);
    public static native int right(int value);
    public static native int join(int value);
    public static native int recover(int value);
    public static native int after(int value);

    public static int route(int key, int input) {
        int saved = before(input);
        try {
            int arm = key == 0 ? left(saved) : right(saved);
            saved = join(arm);
        } catch (IllegalStateException failure) {
            return recover(saved);
        }
        return after(saved);
    }

    public static int loopRoute(int count, int input) {
        int saved = before(input);
        while (count > 0) {
            try {
                saved = (count & 1) == 0 ? left(saved) : right(saved);
                if (saved != 0) {
                    saved = join(saved);
                }
            } catch (IllegalStateException failure) {
                return recover(saved);
            }
            count--;
        }
        return after(saved);
    }

    // The DEX unrolls the six stages with a distinct handler per stage.
    public static int wideRoute(int key, int input) {
        int saved = before(key + input);
        for (int stage = 1; stage <= 6; stage++) {
            try {
                saved = left(saved);
                saved += stage;
                saved = right(saved);
            } catch (IllegalStateException failure) {
                return recover(saved - stage);
            }
            saved = after(saved);
        }
        return saved;
    }
}
