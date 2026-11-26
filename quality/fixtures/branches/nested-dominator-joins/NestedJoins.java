package fixture;

public final class NestedJoins {
    static native int left(int value);
    static native int right(int value);
    static native int join(int value);
    static native int finish(int value);

    public static int run(int key, int value) {
        for (int stage = 0; stage < 5; stage++) {
            if ((key & 1) != 0) {
                value = (key & 2) == 0 ? left(value) : right(value);
            } else if ((key & 4) != 0) {
                return finish(value);
            }
            value = join(value);
            key >>>= 3;
        }
        return finish(value);
    }
}
