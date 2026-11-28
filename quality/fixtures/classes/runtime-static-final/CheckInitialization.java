package fixture;

public final class CheckInitialization {
    private static void event(String text) {
        DexInitEvents.events.add(text);
    }

    private static void compileControls() {
        event("compile:" + (DexConsumer.compileText() == "compiled") + ":" + DexConsumer.compileNumber());
    }

    private static void read(String type, String order) {
        event(order + ":" + type);
        try {
            if (type.equals("text")) {
                String string = DexConsumer.runtimeText();
                event("value:text:" + string + ":" + (string == "runtime"));
            } else {
                event("value:number:" + DexConsumer.runtimeNumber());
            }
        } catch (ExceptionInInitializerError failure) {
            event("error:initializer:" + (failure.getCause() == DexInitEvents.failure));
        } catch (NoClassDefFoundError failure) {
            event("error:prior-initializer");
        }
    }

    public static void main(String[] args) {
        String scenario = args[0];
        DexInitEvents.fail = scenario.startsWith("fail-");
        compileControls();
        if (!scenario.equals("compile-only")) {
            boolean textFirst = scenario.endsWith("-text");
            read(textFirst ? "text" : "number", "first");
            read(textFirst ? "number" : "text", "second");
            compileControls();
        }
        System.out.println(String.join(",", DexInitEvents.events));
    }
}
