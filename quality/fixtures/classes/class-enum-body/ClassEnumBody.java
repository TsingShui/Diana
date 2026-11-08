package fixture;

/** Atomic fixture: enum with constructor state, a method and a constant body. */
public enum ClassEnumBody {
    FIRST(1),
    SECOND(2) {
        @Override
        public int weight() {
            return 20;
        }
    };

    private final int base;

    ClassEnumBody(int base) {
        this.base = base;
    }

    public int weight() {
        return base;
    }

    public int scaled(int factor) {
        return weight() * factor;
    }
}
