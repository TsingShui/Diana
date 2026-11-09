package fixture;

/** Short-circuit tests whose failure path is the condition's common follow. */
public final class TerminalFollow {
    private final String left;
    private final String middle;
    private final String right;

    public TerminalFollow(String left, String middle, String right) {
        this.left = left;
        this.middle = middle;
        this.right = right;
    }

    public static boolean eq(Object first, Object second) {
        return first == null ? second == null : first.equals(second);
    }

    public static boolean probe(int stage) {
        return stage != 0;
    }

    public static void mark(int event) {
        // The fixture keeps a visible effect before each abrupt result arm.
        System.out.println(event);
    }

    public boolean matches(Object other) {
        if (other instanceof TerminalFollow
                && eq(((TerminalFollow) other).left, this.left)
                && eq(((TerminalFollow) other).middle, this.middle)
                && eq(((TerminalFollow) other).right, this.right)) {
            return true;
        }
        return false;
    }

    public boolean effectful(Object other) {
        if (other instanceof TerminalFollow
                && probe(1)
                && eq(((TerminalFollow) other).left, this.left)
                && probe(2)) {
            mark(10);
            return true;
        }
        mark(20);
        return false;
    }
}
