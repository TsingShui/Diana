package fixture;

/**
 * Atomic fixture: every loop whose test sits after the body, next to the
 * head-test shapes that must stay `while`. The DEX of a tail test is one block
 * whose branch returns to its own start; the empty-body variants are the ones
 * that most easily collapse into each other.
 */
public final class LoopTailShapes {
    private static int[] values = new int[4];

    private static Object next() {
        return null;
    }

    private static boolean first() {
        return false;
    }

    private static boolean second() {
        return false;
    }

    /** Head test with an empty body and a side-effecting condition. */
    public static int emptyWhileSideEffect() {
        while (next() != null) {
        }
        return 0;
    }

    /** Tail test whose whole body is the empty block. */
    public static int emptyDoSideEffect() {
        do {
        } while (next() != null);
        return 0;
    }

    /** Head test with an empty body and no side effect. */
    public static int emptyWhile(int limit) {
        int index = 0;
        while (index < limit) {
        }
        return index;
    }

    /** Plain head test. */
    public static int plainWhile(int limit) {
        int index = 0;
        while (index < limit) {
            index++;
        }
        return index;
    }

    /** Plain tail test. */
    public static int plainDo(int limit) {
        int index = 0;
        do {
            index++;
        } while (index < limit);
        return index;
    }

    /** Head test whose update runs in the body, so the shape must stay `while`. */
    public static int emptyForBody(int limit) {
        int index;
        for (index = 0; index < limit; index++) {
        }
        return index;
    }

    /** Statements before a tail test must not enter the loop. */
    public static int initBeforeDo(int limit) {
        int sum = 0;
        int index = 0;
        do {
            sum += index;
            index++;
        } while (index < limit);
        return sum;
    }

    /** Tail test over a compound condition. */
    public static int compoundDo() {
        do {
        } while (first() && second());
        return 0;
    }

    /** Tail test with an early exit in a second block. */
    public static int breakInsideDo(int limit) {
        int index = 0;
        do {
            index++;
            if (index > 10) {
                break;
            }
        } while (index < limit);
        return index;
    }

    /** Head test with an early exit. */
    public static int breakInsideWhile(int limit) {
        int index = 0;
        while (index < limit) {
            if (index == 3) {
                break;
            }
            index++;
        }
        return index;
    }

    /** Tail test whose `continue` must reach the condition. */
    public static int continueInsideDo(int limit) {
        int sum = 0;
        int index = 0;
        do {
            index++;
            if ((index & 1) == 0) {
                continue;
            }
            sum += index;
        } while (index < limit);
        return sum;
    }

    /** Infinite head test narrowed to a tail test by its own exit. */
    public static int infiniteWithBreak(int limit) {
        int index = 0;
        while (true) {
            if (index >= limit) {
                break;
            }
            index++;
        }
        return index;
    }

    /** Head test whose condition needs an array length. */
    public static int scanLength(int[] input) {
        int index = 0;
        while (index < input.length) {
            index++;
        }
        return index;
    }

    /** Tail test that reads the shared array on every condition. */
    public static int scanDo(int limit) {
        int index = 0;
        do {
            index++;
        } while (index < limit && values[index & 3] != 0);
        return index;
    }
}
