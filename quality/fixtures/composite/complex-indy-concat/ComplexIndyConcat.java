package fixture;

/** Atomic fixture: string concatenation through StringConcatFactory (invoke-custom). */
public final class ComplexIndyConcat {
    public static String simple(String name, int count) {
        return name + ":" + count;
    }

    public static String many(String first, String second, int third, long fourth, char fifth) {
        return first + "/" + second + "#" + third + "@" + fourth + "%" + fifth;
    }

    public static String insideLoop(String[] values) {
        String out = "";
        for (String value : values) {
            out = out + "[" + value + "]";
        }
        return out;
    }
}
