package fixture;

/** Atomic fixture: field initializers, an instance block, static init and this(...). */
public final class ClassInitOrder {
    private static int staticCount = 3;
    private static final String LABEL;

    static {
        LABEL = "static";
    }

    private final int first;
    private int second = 2;

    {
        second = second + 1;
    }

    public ClassInitOrder() {
        this(1);
    }

    public ClassInitOrder(int first) {
        this.first = first;
        second = second + first;
    }

    public int total() {
        return first + second + staticCount;
    }

    public String label() {
        return LABEL;
    }
}
