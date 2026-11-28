public final class CheckFloatingBranches {
    private record FloatPair(String name, float left, float right, int order) {}
    private record DoublePair(String name, double left, double right, int order) {}

    // The order is independently specified. Equal and unordered both take
    // the short arm for every tested strict relation.
    private static final FloatPair[] FLOATS = {
        new FloatPair("less", -3.0f, 5.0f, -1),
        new FloatPair("greater", 5.0f, -3.0f, 1),
        new FloatPair("equal", 1.0f, 1.0f, 0),
        new FloatPair("left-nan", Float.NaN, 1.0f, 0),
        new FloatPair("right-nan", 1.0f, Float.NaN, 0),
        new FloatPair("both-nan", Float.NaN, Float.NaN, 0)
    };
    private static final DoublePair[] DOUBLES = {
        new DoublePair("less", 1.0, 1.0000000000000002, -1),
        new DoublePair("greater", 1.0000000000000002, 1.0, 1),
        new DoublePair("equal", 1.0, 1.0, 0),
        new DoublePair("left-nan", Double.NaN, 1.0, 0),
        new DoublePair("right-nan", 1.0, Double.NaN, 0),
        new DoublePair("both-nan", Double.NaN, Double.NaN, 0)
    };

    private static void run(String operation, String scenario, int expected, Runnable work) {
        FloatingBranches.trace = 0;
        work.run();
        System.out.println(operation + ":" + scenario + "\t" + expected + "\t" + FloatingBranches.trace);
    }

    public static void main(String[] args) {
        for (FloatPair pair : FLOATS) {
            String name = pair.name();
            float left = pair.left();
            float right = pair.right();
            boolean less = pair.order() == -1;
            run("floatLow", name, less ? 12345 : 9,
                    () -> FloatingBranches.floatLow(left, right));
            run("floatHigh", name, pair.order() == 1 ? 12345 : 9,
                    () -> FloatingBranches.floatHigh(left, right));
            run("gatedTrue", name, less ? 812345 : 9,
                    () -> FloatingBranches.gated(left, right, true));
            run("gatedFalse", name, less ? 89 : 9,
                    () -> FloatingBranches.gated(left, right, false));
            run("separateReturns", name, less ? 12345 : 9,
                    () -> FloatingBranches.separateReturns(left, right));
        }
        for (DoublePair pair : DOUBLES) {
            run("doubleLow", pair.name(), pair.order() == -1 ? 12345 : 9,
                    () -> FloatingBranches.doubleLow(pair.left(), pair.right()));
            run("doubleHigh", pair.name(), pair.order() == 1 ? 12345 : 9,
                    () -> FloatingBranches.doubleHigh(pair.left(), pair.right()));
        }
        run("integralLow", "less", 12345,
                () -> FloatingBranches.integralLow(Integer.MIN_VALUE, Integer.MAX_VALUE));
        run("integralLow", "greater", 9,
                () -> FloatingBranches.integralLow(Integer.MAX_VALUE, Integer.MIN_VALUE));
        run("integralLow", "equal", 9, () -> FloatingBranches.integralLow(0, 0));
        run("selfEqual", "finite", 9, () -> FloatingBranches.trace = FloatingBranches.selfEqual(1.0));
        run("selfEqual", "nan", 1, () -> FloatingBranches.trace = FloatingBranches.selfEqual(Double.NaN));
    }
}
