import fixture.jvm.ExceptionChains;

class CheckHandlers {
    private static void check(boolean cleanup, int[] values, int divisor, int result, int trace) {
        ExceptionChains input = new ExceptionChains();
        int actual = cleanup ? input.guardedFinally(values, divisor) : input.guarded(values, divisor);
        if (actual != result || input.trace() != trace) {
            throw new AssertionError("result=" + actual + ", trace=" + input.trace());
        }
    }

    private static void checkEscaping(boolean cleanup, int trace) {
        ExceptionChains input = new ExceptionChains();
        try {
            if (cleanup) {
                input.guardedFinally(null, 3);
            } else {
                input.guarded(null, 3);
            }
            throw new AssertionError("missing NullPointerException");
        } catch (NullPointerException expected) {
            if (input.trace() != trace) {
                throw new AssertionError("exception trace=" + input.trace());
            }
        }
    }

    public static void main(String[] args) {
        check(false, new int[]{21}, 3, 7, 14);
        check(false, new int[]{21}, 0, -7, 124);
        check(false, new int[]{}, 0, -8, 15);
        checkEscaping(false, 1);
        check(true, new int[]{21}, 3, 7, 13);
        check(true, new int[]{21}, 0, -7, 123);
        check(true, new int[]{}, 0, -8, 135);
        checkEscaping(true, 13);
    }
}
