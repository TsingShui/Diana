import java.util.ArrayList;
import java.util.List;

final class HierarchyEffects {
    static final RuntimeException ROOT_FAULT = new IllegalStateException("synthetic root fault");
    static final RuntimeException ALIAS_FAULT = new IllegalArgumentException("synthetic alias fault");
    static boolean failRoot;
    static boolean failAlias;
    private static final List<String> EVENTS = new ArrayList<>();

    static void rootStart() {
        record("root:start");
        if (failRoot) {
            record("root:fault");
            throw ROOT_FAULT;
        }
        record("root:complete");
    }

    static int aliasValue() {
        record("alias:start");
        record("alias:reentry:" + ReadStaticHierarchy.inheritedLong());
        if (failAlias) {
            record("alias:fault");
            throw ALIAS_FAULT;
        }
        record("alias:complete");
        return 5;
    }

    static void record(String event) {
        EVENTS.add(event);
    }

    static String trace() {
        return String.join(",", EVENTS);
    }
}
