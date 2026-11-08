package fixture;

/** Atomic fixture: static nested class with its own state. */
public final class ClassInnerStatic {
    public static final class Holder {
        private int value;

        public Holder(int value) {
            this.value = value;
        }

        public int doubled() {
            value += value;
            return value;
        }
    }

    public static Holder holder(int value) {
        return new Holder(value);
    }

    public static int useHolder(int value) {
        return holder(value).doubled();
    }
}
