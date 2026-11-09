package fixture;

/** Behavior explanation for independently authored Smali call-site structures. */
public final class ConcatCapture {
    public static int trace;
    public static void mark(int event) { trace = trace * 31 + event; }
    public static String read(int stage, int fault) {
        mark(10 + stage);
        if (stage == fault) throw new IllegalStateException("read");
        return stage == 0 ? "left" : "right";
    }
    public static String numeric(int left, int right, char mark, boolean flag, long tail) {
        return "" + left + right + mark + flag + tail;
    }
    public static String recipe(String name, int count, char mark) {
        return "tag[" + name + ":" + count + "]" + mark + "\u0001";
    }
    public static String constants(String name) { return name + "7trueA"; }
    public static String floating(float left, double right) { return "" + left + ":" + right; }
    public static String single(String value) { return "" + value; }
    public static String moved(char character, boolean flag) {
        return "" + character + flag;
    }
    public static String charAsInt(char character) { return "" + (int) character; }
    public static String effectful(int fault) {
        String first = read(0, fault);
        String second = read(1, fault);
        return second + ":" + first;
    }
    public static String loop(int limit, int fault) {
        int index = 0;
        String result = "";
        while (index < limit) {
            try {
                String value = read(index, fault);
                result = result + "<" + index + ":" + value + ">";
            } catch (IllegalStateException failure) {
                return "caught";
            }
            index++;
        }
        return result;
    }
    public static String object(Object value, int count) { return "" + value + count; }
    public static String nonfiniteLiteral() { return "" + Float.POSITIVE_INFINITY; }
    // These native explanations denote intentionally invalid/opaque linkage in
    // the actual Smali, not an alternative compile input or a helper model.
    public static native String badRecipe(String name, int count);
    public static native String badBootstrap(String value);
    public static native String unknownFactory(String value, int count);
}
