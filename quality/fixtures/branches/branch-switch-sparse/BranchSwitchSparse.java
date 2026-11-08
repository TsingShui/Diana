package fixture;

/** Atomic fixture: sparse switch with non-contiguous and large case values. */
public final class BranchSwitchSparse {
    private static int state;

    public static int map(int code) {
        switch (code) {
            case -1000:
                return 1;
            case 7:
                return 2;
            case 100:
                state = 1;
                return 3;
            case 65535:
                state = 2;
                return 4;
            default:
                return 0;
        }
    }

    public static int table(int index) {
        switch (index) {
            case 1:
                return 11;
            case 3:
                return 33;
            case 5:
                return 55;
            case 7:
                return 77;
            case 9:
                return 99;
            case 11:
                return 111;
            case 13:
                return 133;
            default:
                return -1;
        }
    }
}
