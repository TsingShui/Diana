import java.util.ArrayList;
import java.util.List;

public final class BuilderConversion {
    public static String precomputed(int mode) {
        Object first = produce(1, mode);
        Object second = produce(2, mode);
        return new StringBuilder().append(first).append(second).toString();
    }

    public static String protectedBuffer(int mode) {
        try {
            Object first = produce(1, mode);
            Object second = produce(2, mode);
            return new StringBuffer().append(first).append(second).toString();
        } catch (IllegalArgumentException failure) {
            return caught("producer", failure);
        } catch (IllegalStateException failure) {
            return caught("conversion", failure);
        }
    }

    private static final List<String> events = new ArrayList<>();
    private static final IllegalArgumentException firstProducer = new IllegalArgumentException();
    private static final IllegalArgumentException secondProducer = new IllegalArgumentException();
    private static final IllegalStateException firstConversion = new IllegalStateException();
    private static final IllegalStateException secondConversion = new IllegalStateException();
    private static final UnsupportedOperationException unmatched = new UnsupportedOperationException();

    private static Object produce(int stage, int mode) {
        events.add("produce:" + stage);
        if (stage == 1 && (mode & 1) != 0) throw firstProducer;
        if (stage == 2 && (mode & 2) != 0) throw secondProducer;
        if (stage == 2 && (mode & 32) != 0) throw unmatched;
        if (stage == 1 && (mode & 16) != 0) return null;
        return new Term(stage, mode);
    }

    private static final class Term {
        private final int stage;
        private final int mode;

        private Term(int stage, int mode) {
            this.stage = stage;
            this.mode = mode;
        }

        @Override public String toString() {
            events.add("convert:" + stage);
            if (stage == 1 && (mode & 4) != 0) throw firstConversion;
            if (stage == 2 && (mode & 8) != 0) throw secondConversion;
            return stage == 1 ? "left" : "/right";
        }
    }

    private static String caught(String path, Throwable fault) {
        events.add("caught:" + path + ":" + identity(fault));
        return path;
    }

    private static String identity(Throwable fault) {
        if (fault == firstProducer) return "first-producer";
        if (fault == secondProducer) return "second-producer";
        if (fault == firstConversion) return "first-conversion";
        if (fault == secondConversion) return "second-conversion";
        if (fault == unmatched) return "unmatched";
        return "foreign";
    }

    public static void main(String[] args) {
        int mode = Integer.parseInt(args[1]);
        try {
            String result = args[0].equals("precomputed") ? precomputed(mode) : protectedBuffer(mode);
            events.add("result:" + result);
        } catch (Throwable failure) {
            events.add("escape:" + identity(failure));
        }
        System.out.println(String.join(",", events));
    }
}
