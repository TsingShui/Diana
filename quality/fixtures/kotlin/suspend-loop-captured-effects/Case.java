package fixture;

/** Explanation of the normal run; README.md defines native helper effects. */
final class SuspendLoop {
    static int total(int limit, int bias) {
        int sum = 0;
        int index = 0;
        int checksum = bias;
        while (index < limit) {
            int argument = prepare(index);
            int captured = sum + checksum;
            record(checksum);
            int loaded = load(argument);
            sum = captured + loaded;
            checksum += index;
            record(sum);
            index++;
        }
        return sum + checksum;
    }

    static native int prepare(int index);
    static native int load(int argument);
    static native void record(int value);
}
