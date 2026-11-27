package fixture.jvm;

import java.util.function.BiFunction;
import java.util.function.Function;

public final class AdaptedLambda {
    private static int trace;

    public static Function<String, String> trim() {
        return argument -> {
            trace = trace * 10 + 1;
            return argument.trim();
        };
    }

    public static BiFunction<String, String[], String> join(String argument) {
        return (first, rest) -> {
            trace = trace * 10 + 2;
            return argument.concat(first).concat(rest[0]);
        };
    }

    public static int trace() {
        return trace;
    }
}
