package fixture.jvm;

import java.util.function.IntUnaryOperator;

public final class CapturedLambda {
    private static int trace;

    public static IntUnaryOperator captured(int digit, long distance) {
        int original = digit;
        return argument -> {
            trace = trace * 10 + original;
            return argument + original + (int) distance;
        };
    }

    public static int trace() {
        return trace;
    }

    public static IntUnaryOperator joined(boolean first, int entry) {
        int chosen;
        if (first) {
            trace += 10;
            chosen = entry;
        } else {
            trace += 20;
            chosen = entry;
        }
        return argument -> chosen + argument;
    }
}
