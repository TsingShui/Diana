package fixture;
// The logical helper receives the frame established by the Smali ABI prefix.
final class OneSite {
    static Object logicalChoose(int seed, Object frame) {
        int argument = prepare(seed);
        int reply = ((Number) load(argument, frame)).intValue();
        record(reply);
        if (reply < 0) return Integer.valueOf(-1);
        observe(new ReadBox(reply, seed));
        return Integer.valueOf(reply + seed);
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
