package fixture.overloads;

import java.util.List;

public final class CheckOverload {
    private static void check(String label, int observed, int expected, int observedTrace, int expectedTrace) {
        System.out.printf("%s value=%d trace=%d%n", label, observed, observedTrace);
        if (observed != expected || observedTrace != expectedTrace) {
            throw new AssertionError(label + " expected " + expected + " with trace " + expectedTrace);
        }
    }

    public static void main(String[] args) {
        check("wide:A", OverloadWitness.wide('A'), 2060, OverloadWitness.observedTrace(), 137578);
        check("narrow:A", OverloadWitness.narrow('A'), 1095, OverloadWitness.observedTrace(), 1264);
        check("wide:FFFE", OverloadWitness.wide((char) 65534), 263936, OverloadWitness.observedTrace(), 137578);
        check("narrow:FFFE", OverloadWitness.narrow((char) 65534), 197502, OverloadWitness.observedTrace(), 1264);
        int generic = OverloadWitness.generic("value", List.of("element"));
        if (generic != 1) throw new AssertionError("generic overload returned " + generic);
    }
}
