package fixture;

// Logical source behavior; Body.smali fixes the continuation ABI and reentry.
final class UnifiedBody {
    Object logicalBody() {
        int argument = ProtocolEffects.prepare();
        Object reply = ProtocolEffects.load(argument, this);
        int number = ((Number) reply).intValue();
        ProtocolEffects.record(number);
        if (number < 0) return Integer.valueOf(-1);
        ReadBox box = new ReadBox(number, number);
        ProtocolEffects.observe(box);
        return reply;
    }
}
final class ProtocolEffects {
    static native int prepare();
    static native Object load(int value, Object continuation);
    static native void record(int value);
    static native void observe(ReadBox value);
}
final class ReadBox {
    int first, second;
    ReadBox(int a, int b) { first = a; second = b; }
}
