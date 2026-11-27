package fixture.jvm;

import java.util.function.IntUnaryOperator;

public final class SyntheticLambda {
    private static int trace;

    public static IntUnaryOperator arithmetic() {
        return value -> value * 3 + 7;
    }

    public static Runnable action() {
        return () -> record(5);
    }

    private static void record(int digit) {
        trace = trace * 10 + digit;
    }

    public static int trace() {
        return trace;
    }
}
