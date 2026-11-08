package fixture;

import java.util.ArrayList;
import java.util.List;

/** Atomic fixture: static, virtual, interface and constructor calls. */
public final class InvokeBasic {
    public interface Counter {
        int next();
    }

    static final class Step implements Counter {
        private int value;

        Step(int start) {
            value = start;
        }

        @Override
        public int next() {
            value += 1;
            return value;
        }
    }

    public static int staticCall(int value) {
        return Math.abs(value);
    }

    public static int virtualCall(Step step) {
        return step.next();
    }

    public static int interfaceCall(Counter counter) {
        return counter.next();
    }

    public static Step constructorCall(int start) {
        return new Step(start);
    }

    public static int manyArguments(int a, int b, int c, int d, int e, int f) {
        return a + b + c + d + e + f;
    }

    public static int listCall(int value) {
        List<Integer> values = new ArrayList<>();
        values.add(value);
        return values.size();
    }
}
