package fixture;

/** Atomic fixture: two-register conditional branches, each side effectful. */
public final class BranchIf2 {
    private static int calls;

    public static int lessThan(int left, int right) {
        if (left < right) {
            calls = 1;
            return left;
        }
        calls = 2;
        return right;
    }

    public static int greaterOrEqual(long left, long right) {
        if (left >= right) {
            calls = 3;
            return 1;
        } else {
            calls = 4;
            return -1;
        }
    }

    public static int equal(int[] left, int[] right) {
        if (left == right) {
            calls = 5;
            return 1;
        }
        calls = 6;
        return 0;
    }
}
