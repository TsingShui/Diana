package fixture;

/** Atomic fixture: single-register conditional branches. */
public final class BranchIfz {
    private static int state;

    public static void whenZero(int value) {
        if (value == 0) {
            state = 1;
        }
    }

    public static void whenNonZero(int value) {
        if (value != 0) {
            state = 2;
        }
    }

    public static int bothPaths(int value) {
        if (value == 0) {
            state = 3;
            return 1;
        }
        state = 4;
        return 0;
    }

    public static void nullCheck(Object input) {
        if (input == null) {
            state = 5;
        }
    }
}
