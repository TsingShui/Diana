package fixture.jvm;

import java.lang.invoke.CallSite;
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.lang.invoke.StringConcatException;
import java.lang.invoke.StringConcatFactory;

public final class CustomBootstrap {
    private CustomBootstrap() {}

    public static CallSite makeConcatWithConstants(
            MethodHandles.Lookup lookup, String name, MethodType type, String recipe,
            Object... constants) throws StringConcatException {
        return StringConcatFactory.makeConcatWithConstants(lookup, name, type, recipe, constants);
    }
}
