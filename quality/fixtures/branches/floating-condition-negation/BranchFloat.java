package fixture;

public final class BranchFloat {
    public static int trace;

    public static void mark(int value) {
        trace = trace * 10 + value;
    }

    public static void fFalls(float left, float right) {
        record(left < right);
    }

    public static void fJoinFalls(float left, float right) {
        record(left < right);
    }

    public static void fJoinBoolean(float left, float right, boolean gate) {
        record(left < right && gate);
    }

    public static void dFalls(double left, double right) {
        record(left > right);
    }

    public static void dJoinFalls(double left, double right) {
        record(left > right);
    }

    public static void iJoinFalls(int left, int right) {
        record(left < right);
    }

    private static void record(boolean large) {
        if (large) {
            for (int marker = 1; marker <= 5; marker++) {
                mark(marker);
            }
        } else {
            mark(9);
        }
    }
}
