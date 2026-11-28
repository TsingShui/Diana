import java.util.ArrayList;
import java.util.List;

/** Executable explanation. Allocation failure is injected at the documented allocation point. */
public final class ArrayOrder {
    private record Marker(String name) {}
    private static final Marker FIRST = new Marker("first");
    private static final Marker SECOND = new Marker("second");
    private static final RuntimeException PRODUCER_FAULT = new RuntimeException("modeled operation fault");
    private static final OutOfMemoryError ALLOCATION_FAULT = new OutOfMemoryError("modeled allocation fault");
    private static final List<String> events = new ArrayList<>();
    private static String failingEvent;
    private static Object[] published;

    static void step(String event) {
        events.add(event);
        if (event.equals(failingEvent)) {
            if (event.startsWith("allocate:")) throw ALLOCATION_FAULT;
            throw PRODUCER_FAULT;
        }
    }

    static Throwable expectedFault() {
        return failingEvent.startsWith("allocate:") ? ALLOCATION_FAULT : PRODUCER_FAULT;
    }

    private static int next(int tag) {
        step("next:" + tag);
        return tag * 10 + 1;
    }

    static Object make(int stage) {
        step("make:" + stage);
        return stage == 10 ? FIRST : SECOND;
    }

    static int[] allocateInts(int length) {
        step("allocate:int:" + length);
        return new int[length];
    }

    static Object[] allocateObjects(int length) {
        step("allocate:object:" + length);
        return new Object[length];
    }

    static Object[] allocateReferences(Object first, Object second) {
        Object[] result = allocateObjects(3);
        result[0] = first;
        result[1] = second;
        result[2] = first;
        return result;
    }

    private static String marker(Object value) {
        if (value == FIRST) return "first";
        if (value == SECOND) return "second";
        if (value == null) return "null";
        throw new AssertionError("unexpected value identity");
    }

    private static void publish(Object[] references) {
        if (references.length != 3 || references[0] != references[2]) {
            throw new AssertionError("incorrect alias in the published array");
        }
        step("publish:" + marker(references[0]) + ":" + marker(references[1]) + ":" + marker(references[2]));
        published = references;
    }

    private static void caught(int stage, Object first, Object second, Throwable error) {
        if (error != expectedFault()) throw new AssertionError("exception identity changed", error);
        step("caught:" + stage + ":" + marker(first) + ":" + marker(second) + ":same-fault");
    }

    public static int[] filled() {
        int first = next(1);
        int second = next(2);
        int[] result = allocateInts(2);
        result[0] = first;
        result[1] = second;
        return result;
    }

    public static int[] stored() {
        int[] result = allocateInts(2);
        result[0] = next(1);
        result[1] = next(2);
        return result;
    }

    public static Object[] staged() {
        int stage = 10;
        Object first = null;
        Object second = null;
        try {
            first = make(stage);
            stage = 20;
            second = make(stage);
            stage = 30;
            Object[] result = allocateReferences(first, second);
            stage = 40;
            publish(result);
            return result;
        } catch (Throwable error) {
            caught(stage, first, second, error);
            return null;
        }
    }

    public static void main(String[] args) {
        failingEvent = args[1];
        try {
            Object result = switch (args[0]) {
                case "filled" -> filled();
                case "stored" -> stored();
                case "staged" -> staged();
                case "protectedFilled" -> ArrayOrderProtected.protectedFilled();
                default -> throw new IllegalArgumentException(args[0]);
            };
            if (result instanceof int[] integers) {
                events.add("return:" + integers[0] + ":" + integers[1]);
            } else if (result instanceof Object[] references) {
                if (args[0].equals("staged") && references != published) {
                    events.add("different-return-array");
                }
                events.add("return:" + marker(references[0]) + ":" + marker(references[1]) + ":" + marker(references[2]));
            } else {
                events.add("return:null");
            }
        } catch (RuntimeException | OutOfMemoryError error) {
            if (error != expectedFault()) throw error;
            events.add("escape:same-fault");
        }
        System.out.println(String.join(",", events));
    }
}

final class ArrayOrderProtected {
    private static Object make(int stage) {
        return ArrayOrder.make(stage);
    }

    private static void caught(int stage, RuntimeException exception) {
        if (exception != ArrayOrder.expectedFault()) throw new AssertionError("exception identity changed", exception);
        ArrayOrder.step("caught:" + stage + ":same-fault");
    }

    public static Object[] protectedFilled() {
        try {
            Object first = make(10);
            Object second = make(20);
            return ArrayOrder.allocateReferences(first, second);
        } catch (RuntimeException error) {
            caught(80, error);
            return null;
        }
    }
}
