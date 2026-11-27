package fixture.jvm;

import java.util.function.IntUnaryOperator;

public final class UnsupportedCapture {
    private static int trace;

    private static int read(int digit) {
        trace = trace * 10 + digit;
        return digit;
    }

    public static IntUnaryOperator computed(int digit) {
        int eager = read(digit);
        return operand -> eager + operand;
    }

    public static IntUnaryOperator overwritten(int digit) {
        int original = digit;
        digit++;
        read(digit);
        return operand -> original + operand;
    }

    public static int trace() {
        return trace;
    }

    public static IntUnaryOperator selected(boolean flag, int left, int right) {
        int chosen;
        if (flag) {
            chosen = left;
        } else {
            chosen = right;
        }
        return argument -> chosen + argument;
    }
}
