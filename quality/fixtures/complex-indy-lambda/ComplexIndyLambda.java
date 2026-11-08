package fixture;

import java.util.ArrayList;
import java.util.List;
import java.util.function.BiFunction;
import java.util.function.Function;
import java.util.function.IntBinaryOperator;
import java.util.function.Supplier;

/** Atomic fixture: lambdas, captured lambdas and every method-reference kind. */
public final class ComplexIndyLambda {
    private int base = 2;

    public String field() {
        return "field";
    }

    public static String staticValue() {
        return "static";
    }

    public Function<String, StringBuilder> constructorRef() {
        return StringBuilder::new;
    }

    public Supplier<List<String>> listFactory() {
        return ArrayList::new;
    }

    public Function<Integer, String> arrayFactory() {
        return size -> {
            String[] values = new String[size];
            return values.length == 0 ? "empty" : "sized";
        };
    }

    public IntBinaryOperator captured(int offset) {
        return (left, right) -> left + right + offset + base;
    }

    public Supplier<String> boundField() {
        return this::field;
    }

    public Supplier<String> unboundStatic() {
        return ComplexIndyLambda::staticValue;
    }

    public BiFunction<String, String, String> stringOps() {
        return String::concat;
    }
}
