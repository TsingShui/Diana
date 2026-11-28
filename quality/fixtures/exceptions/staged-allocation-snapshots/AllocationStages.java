import java.util.ArrayList;
import java.util.List;

public final class AllocationStages {
    public static void run() {
        int stage = 10;
        try {
            int prepared = prepare();
            stage = 15;
            AllocationItem item = new AllocationItem(prepared, argument(stage = 20), stage = 30);
            publish(item);
        } catch (Throwable failure) {
            caught(stage, failure);
        }
    }

    private static final List<String> events = new ArrayList<>();
    private static final RuntimeException beforeFault = new IllegalStateException();
    private static final LinkageError initFault = new LinkageError();
    private static final RuntimeException argumentFault = new IllegalArgumentException();
    private static final RuntimeException constructorFault = new UnsupportedOperationException();
    private static final RuntimeException publishFault = new ArithmeticException();
    private static String fail;

    private static int prepare() {
        events.add("prepare");
        if (fail.equals("prepare")) throw beforeFault;
        return 7;
    }

    static void initialized() {
        events.add("class-init");
        if (fail.equals("init")) throw initFault;
    }

    private static int argument(int stage) {
        events.add("argument:" + stage);
        if (fail.equals("argument")) throw argumentFault;
        return 11;
    }

    static void construct(int prepared, int argument, int stage) {
        events.add("constructor:" + prepared + ":" + argument + ":" + stage);
        if (fail.equals("constructor")) throw constructorFault;
    }

    private static void publish(AllocationItem item) {
        if (item == null) throw new AssertionError("missing initialized item");
        events.add("publish");
        if (fail.equals("publish")) throw publishFault;
    }

    private static void caught(int stage, Throwable actual) {
        events.add("caught:" + stage + ":" + faultName(actual));
    }

    private static String faultName(Throwable actual) {
        if (actual == beforeFault) return "before";
        if (actual == initFault) return "class-init";
        if (actual == argumentFault) return "argument";
        if (actual == constructorFault) return "constructor";
        if (actual == publishFault) return "publish";
        if (actual.getClass() == NoClassDefFoundError.class) return "erroneous";
        return "foreign";
    }

    public static void main(String[] arguments) {
        fail = arguments[0];
        run();
        if (arguments[1].equals("repeat")) {
            fail = "none";
            run();
        }
        System.out.println(String.join(",", events));
    }
}

final class AllocationItem {
    static {
        AllocationStages.initialized();
    }

    AllocationItem(int prepared, int argument, int stage) {
        AllocationStages.construct(prepared, argument, stage);
    }
}
