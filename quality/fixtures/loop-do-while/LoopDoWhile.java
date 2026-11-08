package fixture;

/** Atomic fixture: do-while, including a do-while with break. */
public final class LoopDoWhile {
    private static int state;

    public static int countUp(int limit) {
        int index = 0;
        do {
            index++;
        } while (index < limit);
        return index;
    }

    public static int stopEarly(int limit) {
        int index = 0;
        do {
            state = index;
            index++;
            if (index > limit) {
                break;
            }
        } while (true);
        return index;
    }

    public static int nested(int outerLimit) {
        int total = 0;
        int outer = 0;
        do {
            int inner = 0;
            do {
                total += inner;
                inner++;
            } while (inner < outerLimit);
            outer++;
        } while (outer < outerLimit);
        return total;
    }
}
