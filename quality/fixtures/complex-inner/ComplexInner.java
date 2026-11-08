package fixture;

import java.util.ArrayList;
import java.util.List;

/** Combined fixture: anonymous class, member inner, static nested, generics. */
public final class ComplexInner {
    public interface Visitor {
        int visit(int value);
    }

    public final class Accumulator {
        private int total;

        int add(int value) {
            total += value;
            return total;
        }
    }

    public static final class Bag<T> {
        private final List<T> items = new ArrayList<>();

        public void put(T item) {
            items.add(item);
        }

        public T last() {
            return items.isEmpty() ? null : items.get(items.size() - 1);
        }
    }

    private int base = 3;

    public Visitor adding(final int step) {
        return new Visitor() {
            @Override
            public int visit(int value) {
                base += step;
                return value + base;
            }
        };
    }

    public int runVisitor(int value) {
        Visitor visitor = adding(2);
        int first = visitor.visit(value);
        return visitor.visit(first);
    }

    public int runAccumulator(int[] values) {
        Accumulator accumulator = new Accumulator();
        int total = 0;
        for (int index = 0; index < values.length; index++) {
            total = accumulator.add(values[index]);
        }
        return total;
    }

    public String runBag(String first, String second) {
        Bag<String> bag = new Bag<>();
        bag.put(first);
        bag.put(second);
        return bag.last();
    }
}
