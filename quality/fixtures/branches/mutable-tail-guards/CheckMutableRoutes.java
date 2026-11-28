package fixture;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.stream.Collectors;

final class GuardTrace {
    static final class Fault extends RuntimeException {
        Fault() {
            super(null, null, false, false);
        }
    }

    static final Fault FAULT = new Fault();
    static final List<Integer> calls = new ArrayList<>();
    static final List<String> mismatchedArguments = new ArrayList<>();
    static int seed;
    static int fault;
    static int trace;

    static int step(int actualSeed, int actualFault, int node) {
        calls.add(node);
        if (seed != actualSeed || fault != actualFault) {
            mismatchedArguments.add(actualSeed + ":" + actualFault + ":" + node);
        }
        trace = trace * 31 + node;
        if (node == actualFault) {
            throw FAULT;
        }
        return ((actualSeed >>> (node % 27)) ^ trace ^ (node * 13)) & 1;
    }

    static void reset(int nextSeed, int nextFault, int initialTrace) {
        seed = nextSeed;
        fault = nextFault;
        trace = initialTrace;
        calls.clear();
        mismatchedArguments.clear();
    }
}

public class CheckMutableRoutes {
    public static void main(String[] args) throws Exception {
        for (String row : Files.readAllLines(Path.of(args[0]))) {
            if (row.startsWith("#")) {
                continue;
            }
            String[] cells = row.split("\t");
            String method = cells[0];
            int seed = Integer.parseInt(cells[1]);
            int initial = Integer.parseInt(cells[2]);
            int terminal = Integer.parseInt(cells[3]);
            for (int fault = -1; fault <= terminal; fault++) {
                observe(method, seed, initial, fault);
            }
        }
    }

    private static void observe(String method, int seed, int initial, int fault) {
        GuardTrace.reset(seed, fault, initial);
        String kind;
        String value;
        try {
            int result = switch (method) {
                case "route" -> MutableGuardRoutes.route(seed, fault);
                case "extended" -> MutableGuardRoutes.extended(seed, fault);
                default -> throw new IllegalArgumentException(method);
            };
            kind = "return";
            value = Integer.toString(result);
        } catch (Throwable error) {
            if (error == GuardTrace.FAULT) {
                kind = "fault";
                value = GuardTrace.calls.get(GuardTrace.calls.size() - 1).toString();
            } else {
                kind = "exception";
                value = error.getClass().getName();
            }
        }
        String calls = GuardTrace.calls.stream().map(String::valueOf).collect(Collectors.joining(","));
        System.out.println(String.join("\t", method, Integer.toString(seed), Integer.toString(initial),
                Integer.toString(fault), kind, value, calls, Integer.toString(GuardTrace.trace),
                String.join(",", GuardTrace.mismatchedArguments)));
    }
}
