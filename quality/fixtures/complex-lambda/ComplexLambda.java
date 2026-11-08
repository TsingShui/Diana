package fixture;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.function.Supplier;

/** Combined fixture: lambdas, capturing lambdas and a method reference. */
public final class ComplexLambda {
    private int base = 5;

    public Runnable printer(final String prefix) {
        return () -> System.out.println(prefix + base);
    }

    public Supplier<Integer> counter(final int step) {
        return new Supplier<Integer>() {
            private int value;

            @Override
            public Integer get() {
                value += step;
                return value;
            }
        };
    }

    public int applyTwice(List<Integer> values, int delta) {
        int total = 0;
        for (Integer value : values) {
            total += value + delta;
        }
        return total;
    }

    public List<String> sorted(List<String> values) {
        List<String> copy = new ArrayList<>(values);
        copy.sort(Comparator.naturalOrder());
        return copy;
    }

    public String describe(int value) {
        return "value:" + value;
    }

    public Supplier<String> prefixer() {
        return this::toString;
    }
}
