import java.util.ArrayList;
import java.util.List;

public final class FinallyFamily {
    public static void run(FinallyFamily instance, int token) {
        try {
            stage(1);
            stage(2);
            stage(3);
        } finally {
            instance.cleanup(token);
        }
    }

    private static final List<String> events = new ArrayList<>();
    private static FinallyFamily receiver;
    private static int failStage;
    private static boolean failCleanup;
    private static RuntimeException primary;
    private static RuntimeException cleanupFault;

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

    private static String faultName(Throwable fault) {
        if (fault == primary) return "primary";
        if (fault == cleanupFault) return "cleanup";
        return "unexpected-fault";
    }

    public static void main(String[] arguments) {
        failStage = Integer.parseInt(arguments[0]);
        primary = arguments[1].equals("1") ? new IllegalArgumentException() : new IllegalStateException();
        failCleanup = arguments[3].equals("1");
        cleanupFault = new IllegalStateException();
        receiver = new FinallyFamily();
        try {
            run(receiver, 42);
            events.add("RETURN");
        } catch (Throwable thrown) {
            events.add("THROW:" + faultName(thrown));
        }
        System.out.println(String.join(",", events));
    }
}
