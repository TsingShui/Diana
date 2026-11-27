package fixture.jvm;

public final class SyntheticLambdaCheck {
    public static void main(String[] args) {
        var arithmetic = SyntheticLambda.arithmetic();
        var action = SyntheticLambda.action();
        if (SyntheticLambda.trace() != 0 || arithmetic.applyAsInt(-4) != -5) {
            throw new AssertionError("capture must not execute the helper body");
        }
        action.run();
        if (SyntheticLambda.trace() != 5 || arithmetic.applyAsInt(9) != 34) {
            throw new AssertionError("invocation must use the corresponding helper");
        }
        action.run();
        if (SyntheticLambda.trace() != 55) {
            throw new AssertionError("both invocations must execute the original effect");
        }
    }
}
