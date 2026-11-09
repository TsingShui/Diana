package fixture;

/** Fresh complexity challenge authored after the scalar-use rendering repair. */
public final class ProtectedViews {
    // Model: record the code, then throw the configured throwable at this code.
    static native void mark(int code);

    static char unit(int index) {
        mark(10 + index);
        return (char) (65 + index);
    }
    static int record(char value) {
        int result = 1000 + value;
        mark(result);
        return result;
    }
    static int record(int value) {
        int result = 2000 + value;
        mark(result);
        return result;
    }
    static int route(char[] data, int limit, boolean fromArray) {
        int row = 0;
        int total = 0;
        while (row < limit) {
            char selected;
            if (fromArray) { selected = data[row]; }
            else { selected = unit(row); }
            try {
                total += record(selected);
                total += record((int) selected);
            } catch (IllegalStateException failure) {
                mark(40 + row);
            } finally {
                mark(70 + row);
            }
            row++;
        }
        return total;
    }
    static int composed(int index) { return record(unit(index)); }
    // DEX-only guard unproved(I)I calls record(C) from an unconstrained word.
    // Java would need a new narrowing conversion, absent from that input.
    // Its result for out-of-range words is deliberately not modeled here.
    static int raw(char[] data, boolean first) {
        char selected;
        if (first) { selected = data[0]; }
        else { selected = data[1]; }
        return selected;
    }
}
