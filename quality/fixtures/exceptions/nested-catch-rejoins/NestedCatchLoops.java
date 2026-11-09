package fixture;

/** Nested handler rejoins under a monitor, plus a genuinely foreign arrival. */
public final class NestedCatchLoops {
    public static int run(Object lock, int rows, int mode) {
        int total = 0;
        int row = 0;
        while (row < rows) {
            int column = 0;
            while (column < 3) {
                int code = row * 3 + column;
                synchronized (lock) {
                    int value;
                    try {
                        try {
                            value = LoopCatchFinally.read(code, mode);
                        } catch (NumberFormatException | NullPointerException error) {
                            LoopCatchFinally.mark(20);
                            value = 100;
                        }
                    } finally {
                        LoopCatchFinally.cleanup(code, mode);
                    }
                    total += value;
                }
                column++;
            }
            row++;
        }
        return total;
    }

    /** The Smali shares both catch bodies; a pre-loop fault reaches the join. */
    public static int foreign(int rows, int mode) {
        int row = 0;
        int total = 0;
        try {
            LoopCatchFinally.read(0, mode);
        } catch (NumberFormatException | NullPointerException error) {
            LoopCatchFinally.mark(20);
            total += 100;
            row++;
        }
        while (row < rows) {
            int value;
            try {
                value = LoopCatchFinally.read(row, mode);
            } catch (NumberFormatException | NullPointerException error) {
                LoopCatchFinally.mark(20);
                value = 100;
            }
            total += value;
            row++;
        }
        return total;
    }
}
