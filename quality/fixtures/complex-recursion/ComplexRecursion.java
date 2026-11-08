package fixture;

import java.util.HashMap;
import java.util.Map;

/** Combined fixture: mutual recursion, accumulator recursion and memoisation. */
public final class ComplexRecursion {
    private final Map<Integer, Long> memo = new HashMap<>();

    public static boolean isEven(int value) {
        if (value == 0) {
            return true;
        }
        return isOdd(value - 1);
    }

    public static boolean isOdd(int value) {
        if (value == 0) {
            return false;
        }
        return isEven(value - 1);
    }

    public static int sumTo(int value) {
        return sumTo(value, 0);
    }

    private static int sumTo(int value, int acc) {
        if (value <= 0) {
            return acc;
        }
        return sumTo(value - 1, acc + value);
    }

    public long fibonacci(int index) {
        if (index < 2) {
            return index;
        }
        Long cached = memo.get(index);
        if (cached != null) {
            return cached;
        }
        long value = fibonacci(index - 1) + fibonacci(index - 2);
        memo.put(index, value);
        return value;
    }

    public long factorial(int value) {
        long result = 1;
        for (int index = 2; index <= value; index++) {
            result *= index;
        }
        return result;
    }
}
