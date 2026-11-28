import java.util.ArrayList;
import java.util.List;

public final class BuilderCapacity {
    public static String builder(int capacity, int mode) {
        return new StringBuilder(capacity).append(produce(1, mode)).append(produce(2, mode)).toString();
    }

    public static String buffer(int capacity, int mode) {
        return new StringBuffer(capacity).append(produce(1, mode)).append(produce(2, mode)).toString();
    }

    public static String protectedBuilder(int capacity, int mode) {
        try {
            return new StringBuilder(capacity).append(produce(1, mode)).append(produce(2, mode)).toString();
        } catch (NegativeArraySizeException failure) {
            return caught("capacity", failure);
        } catch (IllegalStateException failure) {
            return caught("producer", failure);
        }
    }

    private static final RuntimeException first = new IllegalStateException();
    private static final RuntimeException second = new IllegalStateException();
    private static final RuntimeException other = new IllegalArgumentException();
    private static final List<String> events = new ArrayList<>();

    private static String produce(int stage, int mode) {
        events.add("producer:" + stage);
        if (mode == stage) throw stage == 1 ? first : second;
        if (mode == 3 && stage == 1) throw other;
        return stage == 1 ? "prefix" : "/suffix";
    }

    private static String caught(String path, Throwable failure) {
        events.add("caught:" + path + ":" + identity(failure));
        return path;
    }

    private static String identity(Throwable failure) {
        if (failure == first) return "first";
        if (failure == second) return "second";
        if (failure == other) return "other";
        if (failure.getClass() == NegativeArraySizeException.class) return "negative-size";
        return "foreign";
    }

    public static void main(String[] arguments) {
        int capacity = Integer.parseInt(arguments[1]);
        int mode = Integer.parseInt(arguments[2]);
        try {
            String result = switch (arguments[0]) {
                case "builder" -> builder(capacity, mode);
                case "buffer" -> buffer(capacity, mode);
                case "protectedBuilder" -> protectedBuilder(capacity, mode);
                default -> throw new AssertionError("unknown subject");
            };
            events.add("result:" + result);
        } catch (Throwable failure) {
            events.add("escape:" + identity(failure));
        }
        System.out.println(String.join(",", events));
    }
}
