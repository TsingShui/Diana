package fixture;

/** Explanation of the authored Smali, not an alternative build input. */
public final class LoopExits {
    public static native void mark(int row);

    public static int nestedBreak(int rows, int columns, int hit) {
        int total = 0;
        for (int row = 0; row < rows; row++) {
            for (int column = 0; column < columns; column++) {
                if (column == hit) {
                    mark(row);
                    total += 10;
                    break;
                }
                total += column;
            }
        }
        return total;
    }

    public static int outerBreak(int rows, int columns, int hit) {
        int total = 0;
        outer:
        for (int row = 0; row < rows; row++) {
            for (int column = 0; column < columns; column++) {
                if (column == hit) {
                    mark(row);
                    total += 10;
                    break outer;
                }
                total += column;
            }
        }
        return total;
    }

    public static int outerContinue(int rows, int columns, int hit) {
        int total = 0;
        outer:
        for (int row = 0; row < rows; row++) {
            for (int column = 0; column < columns; column++) {
                if (column == hit) {
                    mark(row);
                    total += 10;
                    continue outer;
                }
                total += column;
            }
        }
        return total;
    }

    public static int sharedExit(int rows, int columns, int hit) {
        int total = 0;
        for (int row = 0; row < rows; row++) {
            for (int column = 0; column < columns; column++) {
                if (column == hit || column == 1) {
                    mark(row);
                    total += 10;
                    break;
                }
                total += column;
            }
        }
        return total;
    }

    public static int siblingLoop(int limit, int hit) {
        int second = 0;
        for (int first = 0; first < limit; first++) {
            if (first == hit) {
                while (second < limit) {
                    mark(second);
                    second++;
                }
                break;
            }
        }
        return second;
    }

    public static int protectedExit(int rows, int columns, int hit) {
        int total = 0;
        for (int row = 0; row < rows; row++) {
            for (int column = 0; column < columns; column++) {
                if (column == hit) {
                    try {
                        mark(row);
                    } catch (RuntimeException failure) {
                        return -1;
                    }
                    total += 10;
                    break;
                }
                total += column;
            }
        }
        return total;
    }
}
