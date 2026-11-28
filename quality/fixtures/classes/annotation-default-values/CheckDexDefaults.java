package fixture;

import java.lang.reflect.Array;
import java.lang.reflect.Method;
import java.util.StringJoiner;

final class CheckDexDefaults {
    public static void main(String[] args) {
        for (Class<?> type : new Class<?>[] {DexDefaults.class, DexLabel.class}) {
            if (!type.isAnnotation()) {
                throw new AssertionError(type.getName());
            }
            for (Method method : type.getDeclaredMethods()) {
                String key = type.getSimpleName() + "." + method.getName();
                String descriptor = method.getReturnType().descriptorString();
                System.out.println(key + "\t" + descriptor + "\t" + value(method.getDefaultValue()));
            }
        }
    }

    private static String value(Object item) {
        if (item == null) {
            return "null";
        }
        if (item instanceof String string) {
            return quote(string);
        }
        if (item instanceof Integer || item instanceof Boolean) {
            return item.toString();
        }
        if (item instanceof Class<?> type) {
            return "{\"class\":" + quote(type.descriptorString()) + "}";
        }
        if (item instanceof Enum<?> constant) {
            String owner = quote(constant.getDeclaringClass().descriptorString());
            return "{\"enum\":[" + owner + "," + quote(constant.name()) + "," + owner + "]}";
        }
        if (item instanceof DexLabel label) {
            return "{\"annotation\":" + quote(label.annotationType().descriptorString())
                + ",\"values\":{\"rank\":" + label.rank() + ",\"value\":" + quote(label.value()) + "}}";
        }
        if (item.getClass().isArray()) {
            StringJoiner elements = new StringJoiner(",", "[", "]");
            for (int index = 0; index < Array.getLength(item); index++) {
                elements.add(value(Array.get(item, index)));
            }
            return elements.toString();
        }
        throw new AssertionError("Unmodeled value " + item.getClass());
    }

    private static String quote(String text) {
        return "\"" + text.replace("\\", "\\\\").replace("\"", "\\\"")
            .replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t") + "\"";
    }
}
