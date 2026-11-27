import fixture.inline.Expansion;

public final class Verify {
    private static void check(long actual, long expected) {
        if (actual != expected) {
            throw new AssertionError("expected " + expected + ", got " + actual);
        }
    }

    public static void main(String[] args) {
        boolean updated = args[0].equals("after");
        check(Expansion.run(), 7);
        check(Expansion.trace, 1237);
        check(new Expansion().forwarded(4, 10), 24);
        check(Expansion.trace, 164);
        check(Expansion.changedArguments(3, 8), updated ? 14 : 19);
        check(Expansion.trace, updated ? 194 : 149);
        check(Expansion.protectedCall(), 18);
        check(Expansion.trace, 1478);
        Expansion.fail = true;
        check(Expansion.protectedCall(), -1);
        check(Expansion.trace, 14);
        Expansion.fail = false;
        Expansion.trace = 0;
        check(Expansion.unrelated(), updated ? 7 : 12);
        check(Expansion.trace, updated ? 23 : 0);
    }
}
