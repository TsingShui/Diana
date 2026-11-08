package fixture;

import java.util.ArrayList;
import java.util.List;

/** Combined fixture: generic class, generic method, bound, and a bridge method. */
public final class ComplexGenerics {
    public interface Mapper<T> {
        T apply(T value);
    }

    public static final class Identity implements Mapper<String> {
        @Override
        public String apply(String value) {
            return value.trim();
        }
    }

    public static final class Pair<A, B> {
        private final A first;
        private final B second;

        public Pair(A first, B second) {
            this.first = first;
            this.second = second;
        }

        public A first() {
            return first;
        }

        public B second() {
            return second;
        }

        public Pair<B, A> swap() {
            return new Pair<>(second, first);
        }
    }

    public static <T extends Comparable<T>> T largest(List<T> values) {
        T best = values.get(0);
        for (T value : values) {
            if (value.compareTo(best) > 0) {
                best = value;
            }
        }
        return best;
    }

    public static <T> List<T> repeat(T value, int count) {
        List<T> out = new ArrayList<>();
        for (int index = 0; index < count; index++) {
            out.add(value);
        }
        return out;
    }

    public static String applyMapper(Mapper<String> mapper, String value) {
        return mapper.apply(value);
    }
}
