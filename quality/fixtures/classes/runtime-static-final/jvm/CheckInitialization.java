public final class CheckInitialization {
    private static void compiled() {
        String first = ReadRuntimeFields.compiledText();
        String second = ReadRuntimeFields.compiledText();
        InitEffects.log("compile:" + (first == "compiled" && second == first) + ":" + ReadRuntimeFields.compiledNumber());
    }

    private static void read(String kind, String action) {
        InitEffects.log(action + ":" + kind);
        try {
            if (kind.equals("text")) {
                String value = ReadRuntimeFields.runtimeText();
                InitEffects.log("value:text:" + value + ":" + (value == "runtime"));
            } else if (kind.equals("number")) {
                InitEffects.log("value:number:" + ReadRuntimeFields.runtimeNumber());
            } else {
                throw new AssertionError(kind);
            }
        } catch (ExceptionInInitializerError error) {
            InitEffects.log("error:initializer:" + (error.getCause() == InitEffects.FAILURE));
        } catch (NoClassDefFoundError error) {
            InitEffects.log("error:prior-initializer");
        }
    }

    public static void main(String[] args) {
        String scenario = args[0];
        InitEffects.fail = scenario.startsWith("fail-");
        compiled();
        switch (scenario) {
            case "compile-only" -> { }
            case "success-text", "fail-text" -> {
                read("text", "first");
                read("number", "second");
                compiled();
            }
            case "success-number", "fail-number" -> {
                read("number", "first");
                read("text", "second");
                compiled();
            }
            default -> throw new AssertionError(scenario);
        }
        System.out.println(InitEffects.trace());
    }
}
