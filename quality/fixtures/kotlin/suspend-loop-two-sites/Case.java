package fixture;

final class SuspendTwoSites {
    static int total(int limit) {
        int sum = 0;
        int index = 0;
        while (index < limit) {
            int first = load(index);
            record(first);
            if (first < 0) break;
            int second = load(first + 1);
            sum += first + second;
            index++;
        }
        return sum;
    }
    static native int load(int value);
    static native void record(int value);
}
