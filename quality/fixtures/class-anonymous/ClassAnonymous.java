package fixture;

/** Atomic fixture: anonymous class capturing a local and a field. */
public final class ClassAnonymous {
    private int base = 10;

    public Runnable task(final int step) {
        return new Runnable() {
            @Override
            public void run() {
                base += step;
            }
        };
    }

    public int currentBase() {
        return base;
    }
}
