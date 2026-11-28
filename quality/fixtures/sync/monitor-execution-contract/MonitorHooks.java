package fixture;

import java.util.ArrayList;
import java.util.List;

public final class MonitorHooks {
    static final RuntimeException FAULT = new IllegalStateException("authored fault");
    static final List<String> events = new ArrayList<>();

    public static boolean probe(Object receiver, boolean fail) {
        boolean held = Thread.holdsLock(receiver);
        events.add("inside:" + held);
        if (fail) {
            throw FAULT;
        }
        return held;
    }
    public static void probeVoid(Object receiver, boolean fail) {
        probe(receiver, fail);
    }
}
