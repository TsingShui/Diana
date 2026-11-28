import java.util.ArrayList;
import java.util.List;

public final class ConstructorPrefix {
    static boolean failClassInit;
    private static final List<String> events = new ArrayList<>();

    public static int run(int input, boolean fault) {
        int first = prepareFirst(input);
        int prepared = prepareSecond(first, fault);
        return new PrefixValue(prepared + 1, input * 2).value;
    }

    private static int prepareFirst(int input) {
        record("first", input, 0);
        return input + 1;
    }

    private static int prepareSecond(int first, boolean fault) {
        record("second", first, fault ? 1 : 0);
        if (fault) {
            throw new IllegalStateException();
        }
        return first + 2;
    }

    static void record(String event, int left, int right) {
        events.add(event + ":" + left + ":" + right);
    }

    public static void main(String[] arguments) {
        failClassInit = Boolean.parseBoolean(arguments[0]);
        for (int index = 1; index < arguments.length; index += 2) {
            int input = Integer.parseInt(arguments[index]);
            boolean fault = Boolean.parseBoolean(arguments[index + 1]);
            try {
                record("return", run(input, fault), 0);
            } catch (IllegalStateException exception) {
                record("throw-prepare", input, 0);
            } catch (IllegalArgumentException exception) {
                record("throw-constructor", input, 0);
            } catch (ExceptionInInitializerError exception) {
                record("throw-class-init", input, 0);
            } catch (NoClassDefFoundError error) {
                record("throw-erroneous-class", input, 0);
            }
        }
        System.out.println(String.join(",", events));
    }
}

final class PrefixValue {
    static {
        ConstructorPrefix.record("class-init", 0, 0);
        if (ConstructorPrefix.failClassInit) {
            throw new IllegalStateException();
        }
    }

    final int value;

    PrefixValue(int first, int second) {
        ConstructorPrefix.record("construct", first, second);
        if (second == 14) {
            throw new IllegalArgumentException();
        }
        this.value = first * 10 + second;
    }
}
