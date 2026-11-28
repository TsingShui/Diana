import java.util.ArrayList;
import java.util.List;

public final class BackoffRepeated {
    public static void run() {
        while (nextRound()) {
            try {
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
            } catch (Throwable fault) {
                delivered(fault);
            }
        }
    }

    private static final List<String> events = new ArrayList<>();
    private static final long[] INITIAL = {4294967400L, 4294967656L};
    private static final long[] FRESH = {8589934600L, 8589934632L};
    private static final long[] NOW = {8589934732L, 8589934832L};
    private static final RuntimeException[] PRIMARY = {new IllegalStateException(), new IllegalStateException()};
    private static final RuntimeException[] REPLACEMENT = {
        new UnsupportedOperationException(), new UnsupportedOperationException()
    };
    private static final RuntimeException[] HANDLER = {new IllegalArgumentException(), new IllegalArgumentException()};
    private static final InterruptedException[] INTERRUPTED = {new InterruptedException(), new InterruptedException()};
    private static final String[] failures = new String[2];
    private static final int[] cancelFaultAt = new int[2];
    private static int round = -1;
    private static int cancellations;

    private static boolean nextRound() {
        if (round + 1 == failures.length) return false;
        round++;
        cancellations = 0;
        return true;
    }

    private static void record(String event) {
        events.add("r" + (round + 1) + ":" + event);
    }

    private static long initialClock() {
        record("initialClock");
        if (failures[round].equals("initial-clock")) throw PRIMARY[round];
        return INITIAL[round];
    }

    private static long refreshClock() {
        record("refreshClock");
        if (failures[round].equals("refresh-clock")) throw PRIMARY[round];
        return FRESH[round];
    }

    private static void schedule() {
        record("schedule");
        if (failures[round].equals("early")) throw PRIMARY[round];
    }

    private static void save(long value) {
        record("save:" + value);
        if (failures[round].equals("save")) throw PRIMARY[round];
    }

    private static void waitFor() throws InterruptedException {
        record("wait");
        if (failures[round].equals("wait")) throw PRIMARY[round];
        if (failures[round].equals("interrupt") || failures[round].equals("interruption")) throw INTERRUPTED[round];
    }

    private static void interruption(InterruptedException failure) {
        record("interruption:" + faultName(failure));
        if (failures[round].equals("interruption")) throw HANDLER[round];
    }

    private static void reacquire() {
        record("reacquire");
        if (failures[round].equals("late")) throw PRIMARY[round];
    }

    private static void cancel() {
        record("cancel");
        if (++cancellations == cancelFaultAt[round]) throw REPLACEMENT[round];
    }

    private static void diagnostic(long stamp, Throwable failure) {
        record("diagnostic:" + stamp + ":" + (NOW[round] - stamp) + ":" + faultName(failure));
    }

    private static void delivered(Throwable failure) {
        record("delivered:" + faultName(failure));
    }

    private static String faultName(Throwable fault) {
        for (int index = 0; index < failures.length; index++) {
            if (fault == PRIMARY[index]) return "primary-" + (index + 1);
            if (fault == REPLACEMENT[index]) return "replacement-" + (index + 1);
            if (fault == HANDLER[index]) return "handler-" + (index + 1);
            if (fault == INTERRUPTED[index]) return "interrupted-" + (index + 1);
        }
        return "foreign";
    }

    public static void main(String[] arguments) {
        failures[0] = arguments[0];
        cancelFaultAt[0] = Integer.parseInt(arguments[1]);
        failures[1] = arguments[2];
        cancelFaultAt[1] = Integer.parseInt(arguments[3]);
        try {
            run();
            events.add("return");
        } catch (Throwable fault) {
            events.add("escape:" + faultName(fault));
        }
        System.out.println(String.join(",", events));
    }
}
