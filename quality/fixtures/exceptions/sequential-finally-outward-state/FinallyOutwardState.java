import java.util.ArrayList;
import java.util.List;

public final class FinallyOutwardState {
    public static void run(FinallyOutwardState instance, int token, int route) {
        int timestamp = 3;
        try {
            try {
                stage(1);
                if (route == 0) {
                    timestamp = 1;
                } else {
                    timestamp = 2;
                }
                stage(2);
                stage(3);
            } finally {
                instance.cleanup(token);
            }
        } catch (Throwable failure) {
            outward(timestamp, failure);
        }
    }

    private static final List<String> events = new ArrayList<>();
    private static final RuntimeException primary = new IllegalStateException();
    private static final RuntimeException cleanupFault = new UnsupportedOperationException();
    private static final RuntimeException reporterFault = new IllegalArgumentException();
    private static FinallyOutwardState receiver;
    private static int failStage;
    private static boolean failCleanup;
    private static boolean failReporter;

    private static void stage(int stage) {
        events.add("S" + stage);
        if (stage == failStage) {
            throw primary;
        }
    }

    public void cleanup(int token) {
        events.add("C:" + (this == receiver ? "receiver" : "unexpected-receiver") + ":" + token);
        if (failCleanup) {
            throw cleanupFault;
        }
    }

    private static void outward(int timestamp, Throwable failure) {
        events.add("O:" + timestamp + ":" + faultName(failure));
        if (failReporter) {
            throw reporterFault;
        }
    }

    private static String faultName(Throwable fault) {
        if (fault == primary) return "primary";
        if (fault == cleanupFault) return "cleanup";
        if (fault == reporterFault) return "reporter";
        return "unexpected-fault";
    }

    public static void main(String[] arguments) {
        int route = Integer.parseInt(arguments[0]);
        failStage = Integer.parseInt(arguments[1]);
        failCleanup = arguments[2].equals("1");
        failReporter = arguments[3].equals("1");
        receiver = new FinallyOutwardState();
        try {
            run(receiver, 42, route);
            events.add("RETURN");
        } catch (Throwable thrown) {
            events.add("THROW:" + faultName(thrown));
        }
        System.out.println(String.join(",", events));
    }
}
