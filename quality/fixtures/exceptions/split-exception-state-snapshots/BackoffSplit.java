import java.util.ArrayList;
import java.util.List;

public final class BackoffSplit {
    public static void run() {
        long started = initialClock();
        try {
            try {
                schedule();
                try {
                    try {
                        save(started);
                        waitFor();
                    } catch (InterruptedException fault) {
                        interruption(fault);
                    }
                } finally {
                    started = refreshClock();
                    reacquire();
                }
            } finally {
                cancel();
            }
        } catch (Throwable fault) {
            diagnostic(started, fault);
            throw fault;
        }
    }

    private static final List<String> events = new ArrayList<>();
    private static final RuntimeException primary = new IllegalStateException();
    private static final RuntimeException replacement = new UnsupportedOperationException();
    private static final RuntimeException interruptionFault = new IllegalArgumentException();
    private static final InterruptedException interrupted = new InterruptedException();
    private static String fail;
    private static int cancelFaultAt;
    private static int cancellations;

    private static long initialClock() {
        events.add("initialClock");
        if (fail.equals("initial-clock")) throw primary;
        return 4294967400L;
    }

    private static long refreshClock() {
        events.add("refreshClock");
        if (fail.equals("refresh-clock")) throw primary;
        return 8589934600L;
    }

    private static void schedule() {
        events.add("schedule");
        if (fail.equals("early")) throw primary;
    }

    private static void save(long value) {
        events.add("save:" + value);
        if (fail.equals("save")) throw primary;
    }

    private static void waitFor() throws InterruptedException {
        events.add("wait");
        if (fail.equals("wait")) throw primary;
        if (fail.equals("interrupt") || fail.equals("interruption")) throw interrupted;
    }

    private static void interruption(InterruptedException failure) {
        events.add("interruption:" + faultName(failure));
        if (fail.equals("interruption")) throw interruptionFault;
    }

    private static void reacquire() {
        events.add("reacquire");
        if (fail.equals("late")) throw primary;
    }

    private static void cancel() {
        events.add("cancel");
        if (++cancellations == cancelFaultAt) throw replacement;
    }

    private static void diagnostic(long stamp, Throwable failure) {
        events.add("diagnostic:" + stamp + ":" + (8589934732L - stamp) + ":" + faultName(failure));
    }

    private static String faultName(Throwable fault) {
        if (fault == primary) return "primary";
        if (fault == replacement) return "replacement";
        if (fault == interruptionFault) return "interruption-fault";
        if (fault == interrupted) return "interrupted";
        return "foreign";
    }

    public static void main(String[] arguments) {
        fail = arguments[0];
        cancelFaultAt = Integer.parseInt(arguments[1]);
        try {
            run();
            events.add("return");
        } catch (Throwable fault) {
            events.add("escape:" + faultName(fault));
        }
        System.out.println(String.join(",", events));
    }
}
