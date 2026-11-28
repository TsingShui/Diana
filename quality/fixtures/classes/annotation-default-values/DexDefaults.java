package fixture;

import java.lang.annotation.RetentionPolicy;

public @interface DexDefaults {
    Class<?> classChoice() default String[].class;
    int count() default -33;
    int[] empty() default {};
    boolean enabled() default true;
    DexLabel label() default @DexLabel(rank = 4, value = "west");
    DexLabel[] labels() default {
        @DexLabel(rank = -1, value = "north"),
        @DexLabel(rank = 5, value = "east")
    };
    String name() default "row\n\"blue\"";
    RetentionPolicy policy() default RetentionPolicy.RUNTIME;
    int required();
    int[] scores() default {3, -2, 3};
}
