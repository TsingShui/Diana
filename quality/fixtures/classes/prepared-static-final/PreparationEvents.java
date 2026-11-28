package fixture;

import java.util.ArrayList;
import java.util.List;

public final class PreparationEvents {
    static final List<String> trace = new ArrayList<>();
    static final RuntimeException fault = new RuntimeException("parent preparation");
    static boolean fail;

    public static void start() {
        trace.add("start");
    }

    public static void finish() {
        if (fail) {
            trace.add("fault");
            throw fault;
        }
        trace.add("finish");
    }
}
