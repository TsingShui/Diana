public final class FloatingBranches {
    public static int trace;

    static void mark(int digit) {
        trace = trace * 10 + digit;
    }

    public static void floatLow(float left, float right) {
        if (left < right) {
            mark(1);
            mark(2);
            mark(3);
            mark(4);
            mark(5);
        } else {
            mark(9);
        }
    }

    public static void floatHigh(float left, float right) {
        if (left > right) {
            mark(1);
            mark(2);
            mark(3);
            mark(4);
            mark(5);
        } else {
            mark(9);
        }
    }

    public static void doubleLow(double left, double right) {
        if (left < right) {
            mark(1);
            mark(2);
            mark(3);
            mark(4);
            mark(5);
        } else {
            mark(9);
        }
    }

    public static void doubleHigh(double left, double right) {
        if (left > right) {
            mark(1);
            mark(2);
            mark(3);
            mark(4);
            mark(5);
        } else {
            mark(9);
        }
    }

    private static boolean open(boolean value) {
        mark(8);
        return value;
    }

    public static void gated(float left, float right, boolean gate) {
        if (left < right && open(gate)) {
            mark(1);
            mark(2);
            mark(3);
            mark(4);
            mark(5);
        } else {
            mark(9);
        }
    }

    public static void separateReturns(float left, float right) {
        if (left < right) {
            mark(1);
            mark(2);
            mark(3);
            mark(4);
            mark(5);
            return;
        }
        mark(9);
    }

    public static void integralLow(int left, int right) {
        if (left < right) {
            mark(1);
            mark(2);
            mark(3);
            mark(4);
            mark(5);
        } else {
            mark(9);
        }
    }

    public static int selfEqual(double value) {
        return value == value ? 9 : 1;
    }
}
