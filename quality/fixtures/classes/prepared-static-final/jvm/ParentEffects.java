import java.util.ArrayList;
import java.util.List;

final class ParentEffects {
    static final RuntimeException FAULT = new IllegalStateException("original parent initialization fault");
    static boolean fail;
    private static final List<String> EVENTS = new ArrayList<>();

    static void start() {
        record("parent:start");
        if (fail) {
            record("parent:fault");
            throw FAULT;
        }
    }

    static void record(String event) {
        EVENTS.add(event);
    }

    static String trace() {
        return String.join(",", EVENTS);
    }
}
