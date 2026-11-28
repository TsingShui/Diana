import java.util.ArrayList;
import java.util.List;

public final class BuilderIdentity {
    public static final String SUFFIX = "fixed";

    public static void arithmeticBuffer() {
        int first = 127;
        int wrapped = (byte) (first + 2);
        observe(new StringBuffer("number=").append(wrapped).toString());
    }

    public static void staticString() {
        observe(new StringBuilder("field=").append(SUFFIX).toString());
    }

    public static void branchedString(boolean right) {
        String suffix = right ? "right" : "left";
        observe(new StringBuilder("route=").append(suffix).toString());
    }

    public static void literalBuilder() {
        String result = new StringBuilder().append("prefix").append("suffix").toString();
        observe(result);
    }

    public static void singleObjectLiteral() {
        String result = new StringBuilder().append((Object) "payload").toString();
        observe(result);
    }

    public static void primitiveBuffer() {
        String result = new StringBuffer().append(37).append(':').append(true).toString();
        observe(result);
    }

    public static void dynamicSeed(int value) {
        String result = new StringBuilder("value=").append(value).toString();
        observe(result);
    }

    private static final List<String> events = new ArrayList<>();
    private static String existing;

    private static void observe(String result) {
        events.add("text:" + result);
        events.add(result == existing ? "identity:same" : "identity:distinct");
    }

    public static void main(String[] args) {
        existing = args[1].intern();
        switch (args[0]) {
            case "literalBuilder" -> literalBuilder();
            case "singleObjectLiteral" -> singleObjectLiteral();
            case "primitiveBuffer" -> primitiveBuffer();
            case "arithmeticBuffer" -> arithmeticBuffer();
            case "staticString" -> staticString();
            case "branchedString" -> branchedString(Integer.parseInt(args[2]) != 0);
            case "dynamicSeed" -> dynamicSeed(Integer.parseInt(args[2]));
            default -> throw new AssertionError("unknown fixture subject");
        }
        System.out.println(String.join(",", events));
    }
}
