import java.util.ArrayList;
import java.util.List;

final class InitEffects {
    static final RuntimeException FAILURE = new IllegalStateException("authored initializer failure");
    private static final List<String> EVENTS = new ArrayList<>();
    static boolean fail;

    static void begin(String value, int number) {
        log("begin:" + value + ":" + number);
    }

    static void complete() {
        if (fail) {
            log("initializer-fault");
            throw FAILURE;
        }
        log("complete");
    }

    static void log(String event) {
        EVENTS.add(event);
    }

    static String trace() {
        return String.join(",", EVENTS);
    }
}
