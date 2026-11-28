package fixture.overloads;

import java.util.Collection;
import java.util.List;

public final class OverloadWitness {
    private static int trace;
    private final int selected;

    private static void mark(int event) {
        trace = trace * 10 + event;
    }

    private static char once(char unit) {
        mark(1);
        return unit;
    }

    public static int choose(char unit) {
        mark(2);
        return 100 + unit;
    }

    public static int choose(int unit) {
        mark(3);
        return 200 + unit;
    }

    public int select(char unit) {
        mark(4);
        return 300 + unit;
    }

    public int select(int unit) {
        mark(5);
        return 400 + unit;
    }

    public OverloadWitness(char unit) {
        mark(6);
        selected = 500 + unit;
    }

    public OverloadWitness(int unit) {
        mark(7);
        selected = 600 + unit;
    }

    public OverloadWitness(char unit, boolean delegate) {
        this((int) unit);
        mark(8);
    }

    public static int wide(char source) {
        trace = 0;
        char unit = once(source);
        int staticValue = choose((int) unit);
        OverloadWitness direct = new OverloadWitness((int) unit);
        int instanceValue = direct.select((int) unit);
        OverloadWitness delegated = new OverloadWitness(unit, true);
        return staticValue + direct.selected + instanceValue + delegated.selected;
    }

    public static int narrow(char source) {
        trace = 0;
        char unit = once(source);
        int staticValue = choose(unit);
        OverloadWitness direct = new OverloadWitness(unit);
        int instanceValue = direct.select(unit);
        return staticValue + direct.selected + instanceValue;
    }

    public static int observedTrace() {
        return trace;
    }

    public static <T> int selectGeneric(T value, List<T> values) {
        return 1;
    }

    public static <U extends Number> int selectGeneric(String value, Collection<U> values) {
        return 2;
    }

    public static int selectGeneric(Object value, Object values) {
        return 3;
    }

    public static int generic(String value, List<String> values) {
        return selectGeneric(value, values);
    }
}
