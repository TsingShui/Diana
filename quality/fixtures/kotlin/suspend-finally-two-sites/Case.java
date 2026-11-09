package fixture;

final class SuspendFinally {
    static int guarded(int id) {
        try {
            int first = load(id);
            int second = load(first + 1);
            return first + second;
        } finally {
            cleanup(id);
        }
    }
    static native int load(int value);
    static native void cleanup(int id);
}
