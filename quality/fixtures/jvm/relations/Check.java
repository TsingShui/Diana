package fixture.relations;

final class Check {
    public static void main(String[] args) {
        check(14, Relations.outer(0));
        check(11, Relations.virtualDispatch(new Leaf(), 4));
        check(3, Relations.interfaceDispatch(new Leaf(), 4));
        check(5, Relations.interfaceDispatch(new Middle(), 4));
        check(2, Relations.inherited(4));
        check(2, Diamonds.read(new Winning()));
    }

    private static void check(int expected, int actual) {
        if (expected != actual) {
            throw new AssertionError("expected " + expected + ", got " + actual);
        }
    }
}
