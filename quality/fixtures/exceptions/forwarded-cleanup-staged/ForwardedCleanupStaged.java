import java.util.ArrayList;
import java.util.List;

public final class ForwardedCleanupStaged {
    public static void run(ForwardedCleanupStaged left, ForwardedCleanupStaged right, ForwardedCleanupStaged direct, int route) {
        ForwardedCleanupStaged receiver;
        int state;
        long date;
        String tag;
        Throwable pending;
        if (route == 0) {
            try {
                start(11);
                return;
            } catch (Throwable original) {
                receiver = left;
                state = 11;
                date = 4294967297L;
                tag = "forwarded";
                pending = original;
            }
        } else if (route == 1) {
            try {
                start(22);
                return;
            } catch (Throwable original) {
                receiver = right;
                state = 22;
                date = 8589934594L;
                tag = "forwarded";
                pending = original;
            }
        } else {
            try {
                start(33);
                return;
            } catch (Throwable original) {
                receiver = direct;
                state = 33;
                date = 12884901891L;
                tag = "direct";
                pending = original;
            }
        }
        try {
            receiver.cleanup(state, date, tag, pending);
            throw pending;
        } catch (Throwable failure) {
            outward(failure);
        }
    }

    private static final List<String> events = new ArrayList<>();
    private static final RuntimeException LEFT = new IllegalArgumentException();
    private static final RuntimeException RIGHT = new IllegalStateException();
    private static final RuntimeException DIRECT = new SecurityException();
    private static final RuntimeException REPLACEMENT = new UnsupportedOperationException();
    private static final RuntimeException REPORTER = new ArithmeticException();
    private static String mode;
    private final String name;

    private ForwardedCleanupStaged(String name) {
        this.name = name;
    }

    private static String identity(Throwable failure) {
        if (failure == LEFT) return "left-original";
        if (failure == RIGHT) return "right-original";
        if (failure == DIRECT) return "direct-original";
        if (failure == REPLACEMENT) return "replacement";
        if (failure == REPORTER) return "reporter";
        return "foreign";
    }

    private static void start(int state) {
        events.add("start:" + state);
        if (!mode.equals("normal")) throw state == 11 ? LEFT : state == 22 ? RIGHT : DIRECT;
    }

    public void cleanup(int state, long date, String tag, Throwable pending) {
        events.add("cleanup:" + name + ":" + state + ":" + date + ":" + tag + ":" + identity(pending));
        if (mode.equals("replace")) throw REPLACEMENT;
    }

    private static void outward(Throwable failure) {
        events.add("outward:" + identity(failure));
        if (mode.equals("reporter-fault")) throw REPORTER;
    }

    public static void main(String[] arguments) {
        mode = arguments[1];
        try {
            run(new ForwardedCleanupStaged("left"), new ForwardedCleanupStaged("right"), new ForwardedCleanupStaged("direct"), Integer.parseInt(arguments[0]));
            events.add("return");
        } catch (Throwable failure) {
            events.add("escape:" + identity(failure));
        }
        System.out.println(String.join(",", events));
    }
}
