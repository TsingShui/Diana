package fixture;

// Logical source behavior. Smali fixes the initial/resumed ABI correspondence.
final class IntervalLoop {
    static Object logicalTotal(int limit, int seed, Object frame) {
        int sum = 0, index = 0, captured = 0;
        while (index < limit) {
            int argument = prepare(index);
            captured = sum + seed;
            record(seed);
            int reply = ((Number) load(argument, frame)).intValue();
            sum = captured + reply;
            observe(new ReadBox(sum, seed));
            seed += index;
            record(sum);
            index++;
        }
        observe(new ReadBox(captured, seed));
        record(limit);
        return Integer.valueOf(sum + seed);
    }
    static native int prepare(int value);
    static native Object load(int value, Object continuation);
    static native void record(int value);
    static native void observe(ReadBox box);
}
final class ReadBox {
    int first, second;
    ReadBox(int a, int b) { first = a; second = b; }
}
