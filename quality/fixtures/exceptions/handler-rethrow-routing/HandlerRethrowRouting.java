public final class HandlerRethrowRouting {
    public static void run() {
        try {
            while (true) {
                try {
                    setup();
                    break;
                } catch (IllegalArgumentException problem) {
                    if (!retry(problem)) return;
                }
            }
            try {
                schedule();
                try {
                    waitForAlarm();
                } catch (InterruptedException problem) {
                    interrupted(problem);
                }
            } finally {
                cancel();
            }
        } catch (Throwable problem) {
            sessionFailure(problem);
        }
    }

    public static void runWithSecondaryCatch() {
        try {
            while (true) {
                try {
                    setup();
                    break;
                } catch (IllegalArgumentException problem) {
                    if (!retry(problem)) return;
                }
            }
            try {
                schedule();
                try {
                    waitForAlarm();
                } catch (InterruptedException problem) {
                    try {
                        interrupted(problem);
                    } catch (IllegalStateException secondary) {
                        recover(secondary);
                    }
                }
            } finally {
                cancel();
            }
        } catch (Throwable problem) {
            sessionFailure(problem);
        }
    }

    private static final java.util.List<String> events = new java.util.ArrayList<>();
    private static String scenario;
    private static Throwable lastThrown;
    private static int setups;

    private static <T extends Throwable> T failure(T value) {
        lastThrown = value;
        return value;
    }

    private static void setup() {
        events.add("setup");
        if (setups++ == 0 && scenario.startsWith("setup-")) {
            throw failure(new IllegalArgumentException("setup"));
        }
    }

    private static boolean retry(IllegalArgumentException problem) {
        events.add("retry:" + problem.getMessage());
        return !scenario.equals("setup-stop");
    }

    private static void schedule() {
        events.add("schedule");
        if (scenario.startsWith("schedule-")) {
            throw failure(new IllegalArgumentException("schedule"));
        }
    }

    private static void waitForAlarm() throws InterruptedException {
        events.add("wait");
        if (scenario.startsWith("interrupt") || scenario.startsWith("secondary")) {
            throw failure(new InterruptedException("wait"));
        }
    }

    private static void interrupted(InterruptedException problem) {
        events.add("interrupt:" + problem.getMessage());
        if (scenario.startsWith("interrupt-fault")) {
            throw failure(new IllegalArgumentException("interrupt"));
        }
        if (scenario.startsWith("secondary")) {
            throw failure(new IllegalStateException("secondary"));
        }
    }

    private static void recover(IllegalStateException problem) {
        events.add("recover:" + problem.getMessage());
        if (scenario.equals("secondary-fault")) {
            throw failure(new IllegalStateException("recover"));
        }
    }

    private static void cancel() {
        events.add("cancel");
        if (scenario.endsWith("-cancel-fault")) {
            throw failure(new RuntimeException("cancel"));
        }
    }

    private static void sessionFailure(Throwable problem) {
        events.add("session:" + problem.getMessage()
            + ":" + (problem == lastThrown ? "same" : "different"));
    }

    public static void main(String[] arguments) {
        scenario = arguments[1];
        if (arguments[0].equals("nested")) runWithSecondaryCatch();
        else run();
        events.add("return");
        System.out.println(String.join(",", events));
    }
}
