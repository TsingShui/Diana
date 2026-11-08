package fixture;

/** Atomic fixture: switch over an enum, including a default arm. */
public final class BranchSwitchEnum {
    enum Mode {
        FAST,
        SAFE,
        LEGACY
    }

    private static int state;

    public static int apply(Mode mode, int value) {
        switch (mode) {
            case FAST:
                state = 1;
                return value * 2;
            case SAFE:
                return value;
            case LEGACY:
                return value - 1;
            default:
                return -1;
        }
    }
}
