package fixture;

/** Atomic fixture: for with several initialisers/updaters and empty segments. */
public final class LoopForMulti {
    public static int twoPointers(int[] values) {
        int swaps = 0;
        for (int left = 0, right = values.length - 1; left < right; left++, right--) {
            int swap = values[left];
            values[left] = values[right];
            values[right] = swap;
            swaps++;
        }
        return swaps;
    }

    public static int endless(int limit) {
        int index = 0;
        int total = 0;
        for (;;) {
            if (index >= limit) {
                break;
            }
            total += index;
            index++;
        }
        return total;
    }

    public static int emptyUpdate(int limit) {
        int index = 0;
        int total = 0;
        for (; index < limit; ) {
            total += index;
            index += 2;
        }
        return total;
    }

    public static int emptyCondition(int limit) {
        int index = 0;
        for (int step = 1; ; step++) {
            index += step;
            if (index > limit) {
                break;
            }
        }
        return index;
    }
}
