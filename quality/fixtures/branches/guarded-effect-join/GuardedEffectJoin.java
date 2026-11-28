public final class GuardedEffectJoin {
    public static Object run(boolean enabled, boolean publicationFault) {
        try {
            Object selected = enabled ? prepare() : null;
            Object forwarded = selected;
            publish(publicationFault);
            return consume(forwarded);
        } catch (IllegalStateException caught) {
            recover();
            return null;
        }
    }

    public static Object runWithCleanup(boolean enabled, boolean publicationFault) {
        try {
            Object selected = enabled ? prepare() : null;
            Object forwarded = selected;
            publish(publicationFault);
            return consume(forwarded);
        } catch (IllegalStateException caught) {
            recover();
            return null;
        } finally {
            cleanup();
        }
    }

    private static final java.util.List<String> events = new java.util.ArrayList<>();
    private static String fault;

    private static Object prepare() {
        events.add("prepare");
        if ("prepare".equals(fault)) throw new IllegalStateException();
        return "prepared";
    }

    private static void publish(boolean fail) {
        events.add("publish");
        if (fail) throw new IllegalStateException();
    }

    private static Object consume(Object value) {
        events.add("consume:" + value);
        if ("consume".equals(fault)) throw new IllegalArgumentException();
        return value;
    }

    private static void recover() {
        events.add("recover");
        if ("recover".equals(fault)) throw new IllegalArgumentException();
    }

    private static void cleanup() {
        events.add("cleanup");
    }

    public static void main(String[] arguments) {
        boolean enabled = Boolean.parseBoolean(arguments[1]);
        fault = arguments[2];
        boolean publicationFault = "publish".equals(fault) || "recover".equals(fault);
        try {
            Object result = "challenge".equals(arguments[0])
                    ? runWithCleanup(enabled, publicationFault)
                    : run(enabled, publicationFault);
            events.add("return:" + result);
        } catch (IllegalArgumentException escaped) {
            events.add("escape");
        }
        System.out.println(String.join(",", events));
    }
}
