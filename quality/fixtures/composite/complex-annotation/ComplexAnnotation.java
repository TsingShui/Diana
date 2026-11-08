package fixture;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/** Atomic fixture: runtime annotations with array, enum, class and default values. */
@ComplexAnnotation.Marker(
        name = "fixture",
        numbers = {1, 2, 3},
        color = ComplexAnnotation.Color.GREEN,
        type = String.class)
public final class ComplexAnnotation {
    enum Color {
        RED,
        GREEN,
        BLUE
    }

    @Retention(RetentionPolicy.RUNTIME)
    @Target({ElementType.TYPE, ElementType.METHOD, ElementType.FIELD})
    public @interface Marker {
        String name() default "none";

        int[] numbers() default {};

        Color color() default Color.RED;

        Class<?> type() default Object.class;
    }

    @Marker(name = "field")
    private int value;

    @Marker(name = "method", color = Color.BLUE)
    public int read() {
        return value;
    }

    @Marker
    public void write(int next) {
        value = next;
    }
}
