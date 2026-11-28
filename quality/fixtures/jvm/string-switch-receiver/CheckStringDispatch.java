import java.util.function.IntSupplier;

public final class CheckStringDispatch {
    public static void main(String[] args) {
        switch (args[0]) {
            case "custom":
                custom("match", "a", true, 0, "return:100/12");
                custom("reject", "a", false, 0, "return:300/12");
                custom("hash-fault", "a", true, 1, "hash/1");
                custom("equals-fault", "a", true, 2, "equals/12");
                custom("miss-skips-equals", "b", true, 2, "return:300/1");
                check("null", "null/0", null, () -> StringDispatch.custom(null));
                break;
            case "overloaded":
                overloaded("collision-accepted", "BB", true, 0, "return:200/13");
                overloaded("collision-rejected", "BB", false, 0, "return:300/13");
                overloaded("other-hash-accepts", "z", true, 0, "return:100/14");
                overloaded("hash-fault", "BB", true, 1, "hash/1");
                overloaded("overload-fault", "BB", true, 2, "equals/13");
                overloaded("miss-skips-equals", "c", true, 2, "return:300/1");
                check("null", "null/0", null, () -> StringDispatch.overloaded(null));
                break;
            case "genuine":
                check("first", "return:100/0", null, () -> StringDispatch.genuine("a"));
                check("second", "return:200/0", null, () -> StringDispatch.genuine("b"));
                check("miss", "return:300/0", null, () -> StringDispatch.genuine("c"));
                check("null", "null/0", null, () -> StringDispatch.genuine(null));
                break;
            case "genuineCollision":
                check("first", "return:100/0", null, () -> StringDispatch.genuineCollision("Aa"));
                check("second", "return:200/0", null, () -> StringDispatch.genuineCollision("BB"));
                check("miss", "return:300/0", null, () -> StringDispatch.genuineCollision("c"));
                check("null", "null/0", null, () -> StringDispatch.genuineCollision(null));
                break;
            default: throw new AssertionError(args[0]);
        }
    }

    private static void custom(String name, String text, boolean accept, int fault, String expected) {
        SwitchKey key = new SwitchKey(text, accept, fault);
        check(name, expected, key, () -> StringDispatch.custom(key));
    }

    private static void overloaded(String name, String text, boolean accept, int fault, String expected) {
        OverloadedSwitchKey key = new OverloadedSwitchKey(text, accept, fault);
        check(name, expected, key, () -> StringDispatch.overloaded(key));
    }

    private static void check(String name, String expected, SwitchKey key, IntSupplier run) {
        String observed;
        try {
            observed = "return:" + run.getAsInt();
        } catch (RuntimeException failure) {
            if (key == null && failure instanceof NullPointerException) {
                observed = "null";
            } else if (key != null && failure == key.hashFailure) {
                observed = "hash";
            } else if (key != null && failure == key.equalFailure) {
                observed = "equals";
            } else {
                observed = failure.getClass().getName();
            }
        }
        System.out.printf("%s\t%s\t%s/%d%n", name, expected, observed, key == null ? 0 : key.trace);
    }
}
