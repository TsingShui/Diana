public final class CheckDefaultChild {
    private static void compiled() {
        ParentEffects.record("compiled:" + ReadDefaultChild.compiledZero() + ":"
                + (ReadDefaultChild.compiledText() == "interned"));
    }

    private static void prepared(String kind, String point) {
        ParentEffects.record(point + ":" + kind);
        try {
            if (kind.equals("zero")) {
                ParentEffects.record("value:zero:" + ReadDefaultChild.preparedZero());
            } else if (kind.equals("null")) {
                ParentEffects.record("value:null:" + (ReadDefaultChild.preparedNull() == null));
            } else {
                throw new AssertionError(kind);
            }
        } catch (ExceptionInInitializerError error) {
            ParentEffects.record("error:initial:" + (error.getCause() == ParentEffects.FAULT));
        } catch (NoClassDefFoundError error) {
            ParentEffects.record("error:prior");
        }
    }

    public static void main(String[] args) {
        String scenario = args[0];
        ParentEffects.fail = scenario.startsWith("fail-");
        compiled();
        switch (scenario) {
            case "compile-only" -> { }
            case "normal-zero", "fail-zero" -> {
                prepared("zero", "first");
                prepared("null", "second");
                compiled();
            }
            case "normal-null", "fail-null" -> {
                prepared("null", "first");
                prepared("zero", "second");
                compiled();
            }
            default -> throw new AssertionError(scenario);
        }
        if (scenario.startsWith("normal-")) {
            ParentEffects.record("narrow:" + ReadDefaultChild.preparedByte() + ":"
                    + ReadDefaultChild.preparedShort());
        }
        System.out.println(ParentEffects.trace());
    }
}
