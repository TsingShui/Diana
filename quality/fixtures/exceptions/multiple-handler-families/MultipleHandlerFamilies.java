public final class MultipleHandlerFamilies {
    public static void run() {
        try {
            start();
            return;
        } catch (IllegalArgumentException fault) {
            mark(1, fault);
        } catch (IndexOutOfBoundsException fault) {
            mark(2, fault);
        }

        try {
            retry();
            return;
        } catch (UnsupportedOperationException fault) {
            mark(3, fault);
        } catch (ArithmeticException fault) {
            mark(4, fault);
        }

        try {
            await();
        } catch (InterruptedException fault) {
            mark(5, fault);
        }
    }

    public static void runRepeated(int limit) {
        if (limit <= 0) return;

        try {
            start();
            return;
        } catch (IllegalArgumentException fault) {
            mark(1, fault);
        } catch (IndexOutOfBoundsException fault) {
            mark(2, fault);
        }

        try {
            retry();
            return;
        } catch (UnsupportedOperationException fault) {
            mark(3, fault);
        } catch (ArithmeticException fault) {
            mark(4, fault);
        }

        for (int index = 0; index < limit; index++) {
            try {
                await();
            } catch (InterruptedException fault) {
                mark(5, fault);
            }
        }
    }

    private static final java.util.List<String> events = new java.util.ArrayList<>();
    private static Throwable pending;
    private static String first;
    private static String second;
    private static String waiting;
    private static int iteration;

    private static <T extends Throwable> T fault(T exception) {
        pending = exception;
        return exception;
    }

    private static void start() {
        events.add("start");
        switch (first) {
            case "left" -> throw fault(new IllegalArgumentException("left"));
            case "right" -> throw fault(new IndexOutOfBoundsException("right"));
            case "other" -> throw fault(new IllegalStateException("start-other"));
            default -> {}
        }
    }

    private static void retry() {
        events.add("retry");
        switch (second) {
            case "left" -> throw fault(new UnsupportedOperationException("left"));
            case "right" -> throw fault(new ArithmeticException("right"));
            case "other" -> throw fault(new IllegalStateException("retry-other"));
            default -> {}
        }
    }

    private static void await() throws InterruptedException {
        events.add("await:" + iteration);
        char outcome = iteration < waiting.length() ? waiting.charAt(iteration) : 's';
        iteration++;
        if (outcome == 'i') throw fault(new InterruptedException("interrupt"));
        if (outcome == 'x') throw fault(new IllegalStateException("await-other"));
    }

    private static void mark(int route, Throwable exception) {
        events.add("mark:" + route + ":" + exception.getMessage()
            + ":" + (exception == pending ? "same" : "different"));
        if (route == 1 && "hook".equals(second)) {
            throw new UnsupportedOperationException("handler-hook");
        }
    }

    public static void main(String[] arguments) {
        first = arguments[1];
        second = arguments[2];
        waiting = arguments[3];
        try {
            if ("once".equals(arguments[0])) run();
            else runRepeated(Integer.parseInt(arguments[0]));
            events.add("return");
        } catch (RuntimeException fault) {
            events.add("throw:" + fault.getMessage());
        }
        System.out.println(String.join(",", events));
    }
}
