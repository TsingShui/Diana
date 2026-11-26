package fixture;

import java.util.List;

public final class NestedCalls {
    public static <T> T id(T value) {
        return value;
    }

    public static <T extends List<String>> T bounded(T value) {
        return value;
    }

    public static List<?> choose(boolean flag, List<String> typed, List raw) {
        List selected = flag ? bounded(typed) : bounded(raw);
        return (List) id(selected);
    }

    public static String chain14(String value) {
        return id(id(id(id(id(id(id(
               id(id(id(id(id(id(id(value))))))))))))));
    }

    public static <T> String closedStep(String text, T witness) {
        return text;
    }

    public static String closedChain(String text, Object witness) {
        String current = text;
        for (int i = 0; i < 11; i++) {
            current = closedStep(current, witness);
        }
        return current;
    }

    public static <T> T keepAfter(List<String> observed, T next) {
        return next;
    }

    public static List<String> routed(boolean flag, List<String> typed, List raw) {
        return keepAfter(flag ? bounded(typed) : bounded(raw), id(typed));
    }
}
