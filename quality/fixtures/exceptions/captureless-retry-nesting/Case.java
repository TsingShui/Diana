package fixture;

/** Fresh challenge after the captureless private-loop entry repair. */
public final class RetryNesting {
    static native int attempt(int row, int column, int fault);
    static native void advance(int row, int column);

    static int nested(int rows, int columns, int fault) {
        int row = 0, total = 0;
        while (row < rows) {
            int column = 0;
            while (column < columns) {
                try { total += attempt(row, column, fault); }
                catch (IllegalStateException ignored) { }
                advance(row, column);
                column++;
            }
            row++;
        }
        return total;
    }

    static int terminal(int limit, int fault) {
        int index = 0, total = 0;
        while (index < limit) {
            int previous = index;
            index++;
            try { total += attempt(0, previous, fault); }
            catch (IllegalStateException ignored) { continue; }
        }
        return total;
    }

    static int foreign(int limit, int fault) {
        int index = 0, total = 0;
        try { total += attempt(-1, 0, fault); }
        catch (IllegalStateException ignored) { index++; }
        while (index < limit) {
            try { total += attempt(0, index, fault); }
            catch (IllegalStateException ignored) { }
            index++;
        }
        return total;
    }
}
