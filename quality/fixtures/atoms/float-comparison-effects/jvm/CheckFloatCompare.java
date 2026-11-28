import fixture.FloatCompare;
import java.util.function.Supplier;

public final class CheckFloatCompare {
    private record FloatCase(String name, float left, float right, int high, int low, boolean negatedPositive) {}
    private record DoubleCase(String name, double left, double right, int high, int low) {}

    private static final FloatCase[] FLOAT_CASES = {
        new FloatCase("less", -2.0f, 5.0f, -1, -1, true),
        new FloatCase("greater", 5.0f, -2.0f, 1, 1, false),
        new FloatCase("equal", 1.0f, 1.0f, 0, 0, false),
        new FloatCase("pos-neg-zero", 0.0f, -0.0f, 0, 0, false),
        new FloatCase("neg-pos-zero", -0.0f, 0.0f, 0, 0, false),
        new FloatCase("left-nan", Float.NaN, 1.0f, 1, -1, true),
        new FloatCase("right-nan", 1.0f, Float.NaN, 1, -1, true),
        new FloatCase("both-nan", Float.NaN, Float.NaN, 1, -1, true),
        new FloatCase("infinities", Float.NEGATIVE_INFINITY, Float.POSITIVE_INFINITY, -1, -1, true)
    };

    private static final DoubleCase[] DOUBLE_CASES = {
        new DoubleCase("less", -2.0, 5.0, -1, -1),
        new DoubleCase("greater", 1.0000000000000002, 1.0, 1, 1),
        new DoubleCase("equal", 1.0, 1.0, 0, 0),
        new DoubleCase("pos-neg-zero", 0.0, -0.0, 0, 0),
        new DoubleCase("neg-pos-zero", -0.0, 0.0, 0, 0),
        new DoubleCase("left-nan", Double.NaN, 1.0, 1, -1),
        new DoubleCase("right-nan", 1.0, Double.NaN, 1, -1),
        new DoubleCase("both-nan", Double.NaN, Double.NaN, 1, -1)
    };

    private static void observe(String name, Object expected, Object observed) {
        int trace = FloatCompare.trace;
        boolean pass = expected.equals(observed) && trace == 12;
        System.out.println(name + "\t" + (pass ? "PASS" : "FAIL") + "\t" + expected + "\t"
                + observed + "\t" + trace);
    }

    private static void check(String name, Object expected, Supplier<?> compute) {
        FloatCompare.trace = 0;
        Object observed;
        try {
            observed = compute.get();
        } catch (RuntimeException unexpected) {
            observed = "unexpected-throw:" + unexpected.getClass().getSimpleName();
        }
        observe(name, expected, observed);
    }

    private static void base() {
        check("base-one", true, FloatCompare::one);
        check("base-zero", true, FloatCompare::zero);
        check("base-value", 1, FloatCompare::value);
        check("base-nan-normal", true, () -> FloatCompare.nanSkips(null));
        RuntimeException expected = new IllegalStateException("original right-side exception");
        FloatCompare.trace = 0;
        String observed;
        try {
            FloatCompare.nanSkips(expected);
            observed = "no-throw";
        } catch (RuntimeException actual) {
            observed = actual == expected ? "same-fault" : "changed-fault";
        }
        observe("base-nan-throw", "same-fault", observed);
    }

    public static void main(String[] args) {
        base();
        for (FloatCase pair : FLOAT_CASES) {
            String name = pair.name();
            float left = pair.left(), right = pair.right();
            check("float-high-" + name, pair.high(), () -> FloatCompare.floatHigh(left, right));
            check("float-low-" + name, pair.low(), () -> FloatCompare.floatLow(left, right));
            check("predicate-positive-" + name, pair.high() == 1, () -> FloatCompare.highPositive(left, right));
            check("predicate-negative-" + name, pair.low() == -1, () -> FloatCompare.lowNegative(left, right));
            check("nested-operands-" + name, pair.negatedPositive(), () -> FloatCompare.nestedOperands(left, right));
            check("nested-result-" + name, pair.low() == 1, () -> FloatCompare.nestedResult(left, right));
        }
        for (DoubleCase pair : DOUBLE_CASES) {
            String name = pair.name();
            double left = pair.left(), right = pair.right();
            check("double-high-" + name, pair.high(), () -> FloatCompare.doubleHigh(left, right));
            check("double-low-" + name, pair.low(), () -> FloatCompare.doubleLow(left, right));
        }
    }
}
