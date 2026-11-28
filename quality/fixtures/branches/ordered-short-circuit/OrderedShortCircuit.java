public final class OrderedShortCircuit {
    public static boolean run(String input) {
        while (input != null) {
            if (acceptLeft(input) || acceptRight(input)) {
                return true;
            }
            input = next(input);
        }
        return false;
    }

    private static final java.util.List<String> events = new java.util.ArrayList<>();

    private static boolean acceptLeft(String input) {
        events.add("left:" + input);
        if ("left-fault".equals(input)) throw new IllegalArgumentException("left");
        return "both".equals(input);
    }

    private static boolean acceptRight(String input) {
        events.add("right:" + input);
        if ("right-fault".equals(input)) throw new IllegalArgumentException("right");
        return "both".equals(input) || "right".equals(input);
    }

    private static String next(String input) {
        events.add("next:" + input);
        return "chain".equals(input) ? "right" : null;
    }

    public static void main(String[] arguments) {
        try {
            events.add("return:" + run("null".equals(arguments[0]) ? null : arguments[0]));
        } catch (IllegalArgumentException fault) {
            events.add("throw:" + fault.getMessage());
        }
        System.out.println(String.join(",", events));
    }
}
