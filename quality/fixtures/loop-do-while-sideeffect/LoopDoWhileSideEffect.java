package fixture;

import java.util.ArrayDeque;
import java.util.Deque;

/** Atomic fixture: loop conditions that call something on every test. */
public final class LoopDoWhileSideEffect {
    public static int consume(Deque<Integer> queue) {
        int total = 0;
        Integer value;
        while ((value = queue.poll()) != null) {
            total += value;
        }
        return total;
    }

    public static int drain(Deque<Integer> queue) {
        int total = 0;
        do {
            total += queue.size();
        } while (!queue.isEmpty() && queue.poll() != null);
        return total;
    }

    public static int untilZero(int[] values) {
        int index = 0;
        int total = 0;
        do {
            total += values[index];
        } while (values[index++] != 0);
        return total + index;
    }

    public static int retry(int[] attempts) {
        int index = 0;
        int total = 0;
        while (index < attempts.length && attempts[index++] > 0) {
            total += attempts[index - 1];
        }
        return total;
    }
}
