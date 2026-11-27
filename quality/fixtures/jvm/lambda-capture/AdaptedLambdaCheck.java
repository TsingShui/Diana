package fixture.jvm;

import java.util.function.BiFunction;
import java.util.function.Function;

public final class AdaptedLambdaCheck {
    private static void wrongType(Runnable invoke) {
        try {
            invoke.run();
            throw new AssertionError("the original SAM cast must fail");
        } catch (ClassCastException expected) {
            if (AdaptedLambda.trace() != 0) {
                throw new AssertionError("a failed cast must precede the helper body");
            }
        }
    }

    @SuppressWarnings({"rawtypes", "unchecked"})
    public static void main(String[] args) {
        var trim = AdaptedLambda.trim();
        var join = AdaptedLambda.join("entry:");
        if (AdaptedLambda.trace() != 0) {
            throw new AssertionError("capture must not invoke either helper");
        }
        wrongType(() -> ((Function) trim).apply(Integer.valueOf(7)));
        wrongType(() -> ((BiFunction) join).apply(Integer.valueOf(7), new String[] {"valid"}));
        wrongType(() -> ((BiFunction) join).apply("valid first", new Object[] {"wrong array"}));
        if (!trim.apply("  hello ").equals("hello") || AdaptedLambda.trace() != 1) {
            throw new AssertionError("the exact instantiated string reaches the helper");
        }
        if (!join.apply("x", new String[] {"y"}).equals("entry:xy")
                || AdaptedLambda.trace() != 12) {
            throw new AssertionError("the captured input and both invocation arguments keep order");
        }
    }
}
