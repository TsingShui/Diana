import java.lang.annotation.IncompleteAnnotationException;
import java.util.Arrays;
import java.util.Locale;

@Defaults(required = "explicit")
final class Annotated {}

public final class CheckDefaults {
    private static String shape(Object value) {
        if (value == null) return "absent";
        if (value instanceof Float f) return String.format(Locale.ROOT, "%08x", Float.floatToRawIntBits(f));
        if (value instanceof Label label) return label.value() + ":" + label.level();
        if (value instanceof Object[] elements) return Arrays.toString(Arrays.stream(elements).map(CheckDefaults::shape).toArray());
        if (value instanceof int[] elements) return Arrays.toString(elements);
        if (value instanceof Class<?> type) return type.getName();
        if (value instanceof Character c) return Integer.toString(c);
        if (value instanceof String text) return text.replace("\n", "\\n");
        return value.toString();
    }

    public static void main(String[] args) throws Exception {
        for (String field : new String[]{"number", "enabled", "byteValue", "shortValue", "letter", "wide",
                "floatValue", "doubleValue", "title", "token", "choice", "order", "empty", "types", "nested",
                "children", "required"}) {
            System.out.println(field + "\t" + shape(Defaults.class.getDeclaredMethod(field).getDefaultValue()));
        }
        Defaults annotation = Annotated.class.getAnnotation(Defaults.class);
        try {
            System.out.println("annotation-use\t" + annotation.required() + ":" + annotation.number()
                    + ":" + shape(annotation.children()));
        } catch (IncompleteAnnotationException failure) {
            System.out.println("annotation-use\tmissing:" + failure.elementName());
        }
    }
}
