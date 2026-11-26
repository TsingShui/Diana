package fixture;

public final class SiblingExit {
    public static int run(int rows, int mode) {
        int total = 0;
        try {
            for (int row = 0; row < rows; row++) {
                switch (ScopeHooks.select(row, mode)) {
                    case 0:
                        ScopeHooks.skip(row);
                        continue;
                    case 1:
                    case 2:
                        for (int tick = ScopeHooks.start(row); tick > 0; tick--) {
                            ScopeHooks.step(tick);
                        }
                        int value;
                        try {
                            value = ScopeHooks.read(row);
                        } catch (IllegalArgumentException problem) {
                            value = ScopeHooks.recover(row);
                        }
                        total += value;
                        break;
                    default:
                        return total;
                }
            }
            return total;
        } finally {
            ScopeHooks.close(mode);
        }
    }

    public static int sweep(int rows, int mode, int preface) {
        switch (preface) {
            case 0:
            case 1:
                for (int tick = ScopeHooks.start(mode); tick > 0; tick--) {
                    ScopeHooks.step(tick);
                }
                break;
            case 2:
                ScopeHooks.skip(mode);
                break;
            default:
                break;
        }
        int total = 0;
        try {
            for (int row = 0; row < rows; row++) {
                int value;
                try {
                    value = ScopeHooks.read(row);
                } catch (NumberFormatException | NullPointerException problem) {
                    value = ScopeHooks.recover(row);
                }
                total += value;
            }
            return total;
        } finally {
            if (mode == 0) {
                ScopeHooks.skip(rows);
            } else {
                ScopeHooks.step(rows);
                ScopeHooks.close(mode);
            }
        }
    }
}
