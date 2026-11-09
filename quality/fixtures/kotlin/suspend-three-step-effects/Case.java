package fixture;

final class SuspendThree {
    static int walk(int seed) {
        int first = load(seed);
        record(first);
        if (first < 0) return -1;
        int second = load(first + 1);
        record(second);
        if (second == 0) return first;
        int third = load(first + second);
        record(third);
        return first + second + third;
    }
    // normalOnly has the same results but records 777 only when the first load
    // completes immediately; its resume path skips that effect (see README).
    static native int load(int value);
    static native void record(int value);
}
