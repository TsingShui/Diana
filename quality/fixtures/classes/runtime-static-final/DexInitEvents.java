package fixture;

import java.util.ArrayList;
import java.util.List;

public final class DexInitEvents {
    static final List<String> events = new ArrayList<>();
    static final IllegalStateException failure = new IllegalStateException();
    static boolean fail;

    public static void begin(String text, int number) {
        events.add("begin:" + text + ":" + number);
    }

    public static void complete() {
        if (fail) {
            events.add("initializer-fault");
            throw failure;
        }
        events.add("complete");
    }
}
