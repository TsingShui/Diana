public final class PredicateWordJoin {
    public static int select(Object value, int alternative, boolean useAlternative) {
        int count = value instanceof String ? 1 : 0;
        if (useAlternative) count = alternative;
        record(count);
        return count;
    }

    public static int run(Object value, boolean[] steps) {
        int count = value instanceof String ? 1 : 0;
        for (boolean increment : steps) {
            if (increment) count++;
            record(count);
            if (count == 0) absent();
        }
        return count;
    }

    private static final java.util.List<String> events = new java.util.ArrayList<>();

    private static void record(int count) {
        events.add("count:" + count);
    }

    private static void absent() {
        events.add("absent");
    }

    public static void main(String[] arguments) {
        Object value = switch (arguments[1]) {
            case "text" -> "present";
            case "null" -> null;
            default -> Integer.valueOf(12);
        };
        int result;
        if ("select".equals(arguments[0])) {
            result = select(value, Integer.parseInt(arguments[2]), Boolean.parseBoolean(arguments[3]));
        } else {
            String pattern = arguments[2];
            boolean[] steps = new boolean[pattern.length()];
            for (int i = 0; i < steps.length; i++) steps[i] = pattern.charAt(i) == '1';
            result = run(value, steps);
        }
        events.add("return:" + result);
        System.out.println(String.join(",", events));
    }
}
