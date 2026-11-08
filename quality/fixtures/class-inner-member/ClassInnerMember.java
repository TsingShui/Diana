package fixture;

/** Atomic fixture: member inner class with a synthetic accessor to a private field. */
public final class ClassInnerMember {
    private int total;

    public final class Inner {
        private final int step;

        Inner(int step) {
            this.step = step;
        }

        public int apply(int value) {
            total += step;
            return value + total;
        }
    }

    public Inner inner(int step) {
        return new Inner(step);
    }

    public int currentTotal() {
        return total;
    }
}
