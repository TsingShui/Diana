package fixture;

import java.util.ArrayList;
import java.util.List;

public final class InterfaceEffects {
    private static final List<String> EVENTS = new ArrayList<>();

    static String literal() {
        return "first";
    }

    public static String second() {
        add("during:" + RuntimeFace.readFirst() + ":" + RuntimeFace.readSecond());
        return "second";
    }

    static void add(String event) {
        EVENTS.add(event);
    }

    static String trace() {
        return String.join(",", EVENTS);
    }
}
