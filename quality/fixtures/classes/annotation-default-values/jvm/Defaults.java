import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;

@Retention(RetentionPolicy.RUNTIME)
public @interface Defaults {
    int number() default 37;
    boolean enabled() default true;
    byte byteValue() default -7;
    short shortValue() default 300;
    char letter() default '\n';
    long wide() default 9_876_543_210L;
    float floatValue() default -0.0f;
    double doubleValue() default 1.25d;
    String title() default "line\n\"quoted\"";
    Class<?> token() default int[][].class;
    Choice choice() default Choice.LAST;
    int[] order() default {2, 5, 2};
    String[] empty() default {};
    Class<?>[] types() default {String.class, void.class, long[].class, java.time.Instant.class};
    Label nested() default @Label("inside");
    Label[] children() default {@Label("first"), @Label(value = "second", level = 9)};
    String required();
}
