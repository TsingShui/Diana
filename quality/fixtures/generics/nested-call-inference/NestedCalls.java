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
}
