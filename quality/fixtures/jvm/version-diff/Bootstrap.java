package fixture.diff;

import java.lang.invoke.MethodHandles;

public final class Bootstrap {
    public static int calls;

    public static String value(MethodHandles.Lookup lookup, String name, Class<?> type, String text) {
        calls++;
        return text;
    }
}
