package fixture;

final class ReadBox {
    final int first, second;
    ReadBox(int first, int second) { this.first = first; this.second = second; }
}
final class OwnedLoop {
    static int total(int limit, int bias) {
        int sum = 0, index = 0, checksum = bias;
        while (index < limit) {
            int argument = prepare(index);
            int captured = sum + checksum;
            record(checksum);
            int loaded = load(argument);
            sum = captured + loaded;
            observe(new ReadBox(sum, checksum));
            checksum += index;
            record(sum);
            index++;
        }
        return sum + checksum;
    }
    static native int prepare(int index);
    static native int load(int value);
    static native void record(int value);
    static native void observe(ReadBox box);
}
final class OwnedFlow {
    static int walk(int seed) {
        int first = load(seed);
        record(first);
        if (first < 0) return -1;
        int second = load(first + 1);
        record(second);
        observe(new ReadBox(first, second));
        if (second == 0) return first;
        int third = load(first + second);
        record(third);
        return first + second + third;
    }
    static native int load(int value);
    static native void record(int value);
    static native void observe(ReadBox box);
}
