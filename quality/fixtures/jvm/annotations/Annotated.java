package fixture.annotations;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

@Retention(RetentionPolicy.RUNTIME)
@Target({ElementType.TYPE, ElementType.FIELD, ElementType.METHOD, ElementType.PARAMETER})
@interface Rich {
    String text();
    Mode mode();
    Class<?> type();
    int[] numbers();
    Nested nested();
    long count();
    boolean enabled();
}

@Retention(RetentionPolicy.RUNTIME)
@interface Nested {
    String label() default "annotation-default-marker";
}

@Retention(RetentionPolicy.CLASS)
@Target({ElementType.TYPE, ElementType.FIELD, ElementType.METHOD, ElementType.PARAMETER})
@interface Quiet {
    String value();
}

@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.TYPE_USE)
@interface TypeMark {
    String value();
}

enum Mode {
    ALPHA, BETA
}

@Rich(text = "class-visible-marker", mode = Mode.BETA, type = int[].class,
      numbers = {1, -2, 3}, nested = @Nested(label = "nested-marker"),
      count = 123456789012L, enabled = true)
@Quiet("class-invisible-marker")
public final class Annotated {
    public static final String PREPARED = "field-constant-marker";

    @Quiet("field-marker")
    public int saved;

    @Quiet("method-marker")
    public int calculate(
        @Quiet("first-parameter-marker") long first,
        @Quiet("second-parameter-marker") int second
    ) {
        return (int) first + second + saved;
    }
}

final class TypeUsage {
    public @TypeMark("type-use-marker") String value() {
        return "plain-method-body";
    }
}

enum Hidden {
    ITEM(3);

    Hidden(@Quiet("enum-parameter-marker") int value) {}
}
