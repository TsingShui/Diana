import java.util.ArrayList;
import java.util.List;

public final class FinallyTyped {
    public static void run(FinallyTyped instance, int token) {
        try {
            stage(1);
            try {
                stage(2);
            } catch (IllegalArgumentException fault) {
                onTyped(fault);
            }
            stage(3);
        } finally {
            instance.cleanup(token);
        }
    }

    private static final List<String> events = new ArrayList<>();
    private static FinallyTyped receiver;
    private static int failStage;
    private static boolean failHandler;
    private static boolean failCleanup;
    private static RuntimeException primary;
    private static RuntimeException handlerFault;
    private static RuntimeException cleanupFault;

    private static void stage(int stage) {
        events.add("S" + stage);
        if (stage == failStage) {
            throw primary;
        }
    }

    private static void onTyped(IllegalArgumentException delivered) {
        events.add("H:" + faultName(delivered));
        if (failHandler) {
            throw handlerFault;
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
        if (fault == handlerFault) return "handler";
        if (fault == cleanupFault) return "cleanup";
        return "unexpected-fault";
    }

    public static void main(String[] arguments) {
        failStage = Integer.parseInt(arguments[0]);
        primary = arguments[1].equals("1") ? new IllegalArgumentException() : new IllegalStateException();
        failHandler = arguments[2].equals("1");
        failCleanup = arguments[3].equals("1");
        handlerFault = new IllegalStateException();
        cleanupFault = new IllegalStateException();
        receiver = new FinallyTyped();
        try {
            run(receiver, 42);
            events.add("RETURN");
        } catch (Throwable thrown) {
            events.add("THROW:" + faultName(thrown));
        }
        System.out.println(String.join(",", events));
    }
}
