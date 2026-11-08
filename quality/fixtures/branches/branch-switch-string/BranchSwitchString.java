package fixture;

/** Atomic fixture: switch over String, with effects in two branches. */
public final class BranchSwitchString {
    private static String last;

    public static int dispatch(String operation) {
        switch (operation) {
            case "add":
                last = operation;
                return 1;
            case "sub":
                return 2;
            case "mul":
                last = "mul";
                return 3;
            default:
                return 0;
        }
    }

    public static String describe(String key) {
        switch (key) {
            case "a":
                return "first";
            case "b":
            case "c":
                return "grouped";
            default:
                return "other";
        }
    }
}
