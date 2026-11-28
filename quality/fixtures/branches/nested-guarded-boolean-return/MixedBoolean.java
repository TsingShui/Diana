import java.util.function.BooleanSupplier;

public final class MixedBoolean {
    public static boolean mixed() {
        return BooleanEffects.first() && (!BooleanEffects.second() || BooleanEffects.third())
            || !BooleanEffects.fourth();
    }

    public static boolean effectful() {
        if (BooleanEffects.first()) {
            BooleanEffects.mark();
            return true;
        }
        return !BooleanEffects.second();
    }

    public static void main(String[] args) {
        BooleanSupplier body = switch (args[0]) {
            case "mixed" -> MixedBoolean::mixed;
            case "control" -> MixedBoolean::effectful;
            default -> throw new IllegalArgumentException(args[0]);
        };
        BooleanEffects.report(body, Integer.parseInt(args[1]), Integer.parseInt(args[2]));
    }
}

final class BooleanEffects {
    public static int trace;
    public static int flags;
    public static int throwAt;
    public static IllegalStateException failure;

    private static boolean observe(int event) {
        trace = trace * 10 + event;
        if (event == throwAt) throw failure;
        return (flags & (1 << (event - 1))) != 0;
    }

    public static boolean first() { return observe(1); }
    public static boolean second() { return observe(2); }
    public static boolean third() { return observe(3); }
    public static boolean fourth() { return observe(4); }
    public static void mark() { observe(5); }

    static void report(BooleanSupplier body, int flags, int throwAt) {
        BooleanEffects.trace = 0;
        BooleanEffects.flags = flags;
        BooleanEffects.throwAt = throwAt;
        IllegalStateException supplied = new IllegalStateException("supplied");
        BooleanEffects.failure = supplied;
        String outcome;
        try {
            outcome = Boolean.toString(body.getAsBoolean());
        } catch (Throwable thrown) {
            outcome = thrown == supplied ? "same-fault" : "different-fault";
        }
        System.out.printf("%s,%d%n", outcome, BooleanEffects.trace);
    }
}
