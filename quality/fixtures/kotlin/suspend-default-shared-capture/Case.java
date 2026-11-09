package fixture;

final class SuspendCapturedDefault {
    static int choose(int seed) {
        int first = load(seed);
        record(first);
        if (first < 0) return -1;
        int second = load(first + 1);
        record(second);
        return first + second + seed;
    }
    static native int load(int value);
    static native void record(int value);
}
