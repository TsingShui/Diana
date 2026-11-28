package fixture;

public final class CheckFillData {
    private static String ints(int[] data) {
        if (data == null) {
            return "null";
        }
        StringBuilder text = new StringBuilder();
        for (int value : data) {
            if (text.length() > 0) {
                text.append('/');
            }
            text.append(value);
        }
        return text.toString();
    }

    private static String rawFloats(float[] data) {
        StringBuilder text = new StringBuilder();
        for (float value : data) {
            if (text.length() > 0) {
                text.append('/');
            }
            text.append(String.format("%08x", Float.floatToRawIntBits(value)));
        }
        return text.toString();
    }

    public static void main(String[] args) {
        String scenario = args[0];
        ArithmeticException fault = new ArithmeticException("independent fill receiver fault");
        int[] ints = switch (scenario) {
            case "short", "protected-short" -> new int[] {7};
            case "empty", "null", "protected-null", "receiver-null" -> null;
            case "empty-array" -> new int[0];
            case "floats", "float-bits" -> null;
            default -> new int[] {7, 8, 9, 10};
        };
        float[] floats = switch (scenario) {
            case "floats" -> new float[] {7, 8, 9};
            case "float-bits" -> new float[] {7, 8, 9, 10};
            default -> null;
        };
        FillData.shared = ints;
        String outcome = "return:void";
        try {
            switch (scenario) {
                case "short", "null", "int-prefix" -> FillData.fillTwo(ints);
                case "empty", "empty-array", "empty-with-tail" -> FillData.fillEmpty(ints);
                case "floats" -> FillData.fillFloat(floats);
                case "float-bits" -> FillData.fillFloatBits(floats);
                case "receiver", "receiver-null" -> FillData.fillFromReceiver();
                case "protected-normal", "protected-short", "protected-null", "protected-fault" ->
                        outcome = "return:" + FillData.protectedFill(ints,
                                scenario.equals("protected-fault") ? fault : null);
                default -> throw new AssertionError(scenario);
            }
        } catch (RuntimeException lost) {
            outcome = "throw:" + lost.getClass().getSimpleName();
        }
        String contents = floats == null ? ints(ints) : rawFloats(floats);
        String caught = FillData.caught == null ? "none" : FillData.caught == fault ? "exact" : "other";
        System.out.println(outcome + ",contents:" + contents + ",trace:" + FillData.trace
                + ",caught:" + caught + ",alias:" + (FillData.shared == ints));
    }
}
