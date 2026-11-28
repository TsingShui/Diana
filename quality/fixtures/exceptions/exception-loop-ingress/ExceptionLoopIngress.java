public final class ExceptionLoopIngress {
    public static String run() {
        String buffered = null;
        for (String line = next(); line != null; line = next()) {
            if (buffered == null) {
                if (partial(line)) {
                    buffered = line;
                } else {
                    try {
                        parse(line);
                    } catch (IllegalArgumentException | IndexOutOfBoundsException ignored) {
                    }
                }
            } else {
                String joined = append(buffered, line);
                if (partial(joined)) {
                    buffered = joined;
                } else {
                    try {
                        parse(joined);
                        buffered = null;
                    } catch (IllegalArgumentException ignored) {
                        buffered = null;
                    } catch (IndexOutOfBoundsException ignored) {
                        buffered = joined;
                    }
                }
            }
        }
        return buffered;
    }

    private static final java.util.List<String> events = new java.util.ArrayList<>();
    private static java.util.Iterator<String> lines;
    private static String fault;
    private static String faultValue;

    private static String next() {
        String line = lines.hasNext() ? lines.next() : null;
        events.add("next:" + line);
        return line;
    }

    private static boolean partial(String line) {
        events.add("partial:" + line);
        return line.endsWith("-");
    }

    private static String append(String buffered, String line) {
        events.add("append:" + buffered + "+" + line);
        if ("append-bounds".equals(fault)) {
            events.add("fault:append-bounds");
            throw new IndexOutOfBoundsException("append-bounds");
        }
        return buffered + line;
    }

    private static void parse(String line) {
        events.add("parse:" + line);
        if (!line.equals(faultValue)) return;
        switch (fault) {
            case "shape" -> {
                events.add("fault:shape");
                throw new IllegalArgumentException("shape");
            }
            case "bounds" -> {
                events.add("fault:bounds");
                throw new IndexOutOfBoundsException("bounds");
            }
            case "unexpected" -> {
                events.add("fault:unexpected");
                throw new IllegalStateException("unexpected");
            }
            default -> {}
        }
    }

    public static void main(String[] arguments) {
        fault = arguments[0];
        faultValue = arguments[1];
        lines = java.util.Arrays.asList(arguments).subList(2, arguments.length).iterator();
        try {
            events.add("return:" + run());
        } catch (RuntimeException error) {
            events.add("throw:" + error.getMessage());
        }
        System.out.println(String.join(",", events));
    }
}
