public final class RuntimeConcat {
    private static final String COMPILE_PREFIX = "prefix";
    private static final int COMPILE_NUMBER = 7;
    private static final String BLANK_PREFIX;
    private static final int BLANK_NUMBER;
    private static final String RUNTIME_PREFIX = new String("prefix");
    private static int mutableNumber = 7;

    static {
        BLANK_PREFIX = "prefix";
        BLANK_NUMBER = 7;
    }

    public static String literalBuilder() {
        return new StringBuilder().append("prefix").append("suffix").toString();
    }

    public static String literalBuffer() {
        return new StringBuffer().append("value=").append(7).toString();
    }

    public static String equalBranchBuilder(boolean chooseFirst) {
        String term;
        if (chooseFirst) {
            term = "suffix";
        } else {
            term = "suffix";
        }
        return new StringBuilder().append("prefix").append(term).toString();
    }

    public static String equalBranchBuffer(boolean chooseFirst) {
        int term;
        if (chooseFirst) {
            term = 7;
        } else {
            term = 7;
        }
        return new StringBuffer().append("value=").append(term).toString();
    }

    public static String blankStringIndy() {
        return BLANK_PREFIX + "suffix";
    }

    public static String blankPrimitiveIndy() {
        return "value=" + BLANK_NUMBER;
    }

    public static String blankFieldBuilder() {
        return new StringBuilder().append(BLANK_PREFIX).append("suffix").toString();
    }

    public static String compilerStringLiteral() {
        return COMPILE_PREFIX + "suffix";
    }

    public static String compilerPrimitiveLiteral() {
        return "value=" + COMPILE_NUMBER;
    }

    public static String compilerFieldsBuilder() {
        return new StringBuilder().append(COMPILE_PREFIX).append(COMPILE_NUMBER).toString();
    }

    public static String runtimeFieldIndy() {
        return RUNTIME_PREFIX + "suffix";
    }

    public static String mutableFieldIndy() {
        return "value=" + mutableNumber;
    }

    public static String runtimeBuilder(int number) {
        return new StringBuilder().append("value=").append(number).toString();
    }

    public static String runtimeBuffer(String suffix) {
        return new StringBuffer().append("prefix").append(suffix).toString();
    }

    public static String runtimeIndy(String prefix) {
        return prefix + "suffix";
    }
}
