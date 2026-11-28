package fixture.jvm;

/** Authored JVM debug-local anchors for returned primitive, array and reference values. */
public final class TypedLocals {
    public static long runningTotal(int remaining) {
        long total = 0L;
        while (remaining > 0) {
            total += remaining;
            remaining--;
        }
        return total;
    }

    public static int[] copyWindow(int length) {
        int[] result = new int[length];
        for (int index = 0; index < length; index++) {
            result[index] = index + 1;
        }
        return result;
    }

    public static String keepLast(String[] values) {
        String last = "";
        for (int index = 0; index < values.length; index++) {
            last = values[index];
        }
        return last;
    }

    public static double runningFraction(int remaining) {
        double total = 0.0;
        while (remaining > 0) {
            total += remaining / 2.0;
            remaining--;
        }
        return total;
    }
}
