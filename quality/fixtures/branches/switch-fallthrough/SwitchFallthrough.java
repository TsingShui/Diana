package fixture;

/** Shared case labels, linear fallthrough and guards against unsafe entry copies. */
public final class SwitchFallthrough {
    public static int state;
    public static float observed;

    public static void mark(int event) {
        state = state * 10 + event;
    }

    public static void observeFloat(float value) {
        observed = value;
    }

    public static int mixedBits(int code) {
        observeFloat(1.0f);
        int result = 1065353216;
        switch (code) {
            case 10:
                result += 1;
            case 11:
                result += 2;
                break;
            default:
                result = -1;
        }
        return result;
    }

    public static int shared(int code) {
        switch (code) {
            case 1:
            case 2:
                mark(1);
                return 11;
            case 3:
                mark(3);
                return 33;
            default:
                return -1;
        }
    }

    public static int chain(int code) {
        int result = 0;
        switch (code) {
            case 10:
                mark(1);
                result += 1;
            case 11:
                mark(2);
                result += 2;
                break;
            case 12:
                result = 3;
                mark(3);
                break;
            default:
                result = -1;
        }
        return result;
    }

    public static int parameterSeed(int code, int seed) {
        switch (code) {
            case 10:
                seed += 1;
            case 11:
                seed += 2;
                break;
            default:
                seed = -1;
        }
        return seed;
    }

    public static int emptyDefault(int code) {
        int result = 0;
        switch (code) {
            case 1:
                result = 3;
                mark(1);
                break;
            case 2:
                result = 5;
                mark(2);
                break;
            default:
                break;
        }
        return result;
    }

    public static int conditional(int code, int flag) {
        switch (code) {
            case 1:
                mark(1);
                if (flag == 0) break;
            case 2:
                mark(2);
                break;
            default:
                mark(9);
        }
        return state;
    }

    public static int sharedTail(int code) {
        switch (code) {
            case 1:
                mark(1);
                mark(3);
                break;
            case 2:
                mark(2);
            case 3:
                mark(3);
                break;
            default:
                mark(9);
        }
        return state;
    }

    public static long wide(int code) {
        long result = 0;
        switch (code) {
            case 10:
                result += 1;
            case 11:
                result += 2;
                break;
            default:
                result = -1;
        }
        return result;
    }
}
