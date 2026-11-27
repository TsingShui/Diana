package fixture.jvm;

public final class ExceptionChains {
    private int trace = 1;

    public int guarded(int[] values, int divisor) {
        try {
            int result;
            try {
                result = values[0] / divisor;
            } catch (ArithmeticException error) {
                mark(2);
                result = -7;
            }
            mark(4);
            return result;
        } catch (ArrayIndexOutOfBoundsException error) {
            mark(5);
            return -8;
        }
    }

    public int guardedFinally(int[] values, int divisor) {
        try {
            try {
                return values[0] / divisor;
            } catch (ArithmeticException error) {
                mark(2);
                return -7;
            } finally {
                mark(3);
            }
        } catch (ArrayIndexOutOfBoundsException error) {
            mark(5);
            return -8;
        }
    }

    public int trace() {
        return trace;
    }

    private void mark(int digit) {
        trace = trace * 10 + digit;
    }
}
