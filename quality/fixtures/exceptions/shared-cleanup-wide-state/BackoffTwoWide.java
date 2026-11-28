import java.util.ArrayList;
import java.util.List;

public final class BackoffTwoWide {
    public static void run() {
        long started = initialClock();
        long marker = 12884901992L;
        try {
            try {
                schedule();
                try {
                    try {
                        save(started);
                        marker = 17179869364L;
                        waitFor();
                    } catch (InterruptedException fault) {
                        interruption(fault);
                    }
                } finally {
                    started = refreshClock();
                    marker = reacquire();
                }
            } finally {
                cancel();
            }
            complete(started, marker);
        } catch (Throwable fault) {
            diagnostic(started, marker, fault);
            throw fault;
        }
    }

    private static final List<String> events = new ArrayList<>();
    private static final RuntimeException primary = new IllegalStateException();
    private static final RuntimeException replacement = new UnsupportedOperationException();
    private static final RuntimeException interruptionFault = new IllegalArgumentException();
    private static final RuntimeException refreshReplacement = new IndexOutOfBoundsException();
    private static final RuntimeException completionFault = new ArithmeticException();
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
        if (fail.equals("save-and-refresh")) throw refreshReplacement;
        return 8589934600L;
    }

    private static void schedule() {
        events.add("schedule");
        if (fail.equals("early")) throw primary;
    }

    private static void save(long value) {
        events.add("save:" + value);
        if (fail.equals("save") || fail.equals("save-and-refresh")) throw primary;
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

    private static long reacquire() {
        events.add("reacquire");
        if (fail.equals("late")) throw primary;
        return 21474836620L;
    }

    private static void cancel() {
        events.add("cancel");
        if (++cancellations == cancelFaultAt) throw replacement;
    }

    private static void complete(long stamp, long marker) {
        events.add("complete:" + stamp + ":" + marker);
        if (fail.equals("complete")) throw completionFault;
    }

    private static void diagnostic(long stamp, long marker, Throwable failure) {
        events.add("diagnostic:" + stamp + ":" + marker + ":" + faultName(failure));
    }

    private static String faultName(Throwable fault) {
        if (fault == primary) return "primary";
        if (fault == replacement) return "replacement";
        if (fault == interruptionFault) return "interruption-fault";
        if (fault == refreshReplacement) return "refresh-replacement";
        if (fault == completionFault) return "completion-fault";
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
