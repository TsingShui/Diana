package fixture;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;

/** Atomic fixture: bounded generics, a bridge method and wildcard signatures. */
public final class ClassBridge {
    public static final class Box<T extends Comparable<T>> implements Comparator<T> {
        private final List<T> items = new ArrayList<>();

        public void add(T item) {
            items.add(item);
        }

        public T largest() {
            T best = items.get(0);
            for (T item : items) {
                if (item.compareTo(best) > 0) {
                    best = item;
                }
            }
            return best;
        }

        @Override
        public int compare(T left, T right) {
            return left.compareTo(right);
        }
    }

    public static int countAbove(List<? extends Number> values, double limit) {
        int count = 0;
        for (Number value : values) {
            if (value.doubleValue() > limit) {
                count++;
            }
        }
        return count;
    }

    public static <T> void copyInto(List<? super T> target, List<T> source) {
        target.addAll(source);
    }
}
