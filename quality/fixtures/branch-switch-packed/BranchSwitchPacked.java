package fixture;

/** Atomic fixture: packed switch with fallthrough, negative cases and default. */
public final class BranchSwitchPacked {
    private static int state;

    public static int classify(int code) {
        switch (code) {
            case -1:
                return 0;
            case 0:
            case 1:
                state = 1;
                return 10;
            case 2:
                state = 2;
                // fallthrough
            case 3:
                return 30 + state;
            default:
                state = 9;
                return -1;
        }
    }

    public static int clustered(int code) {
        int result = 0;
        switch (code) {
            case 10:
                result += 1;
            case 11:
                result += 2;
                break;
            case 12:
                result = 3;
                break;
            default:
                result = -1;
        }
        return result;
    }
}
