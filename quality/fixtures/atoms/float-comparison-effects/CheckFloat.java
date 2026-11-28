package fixture;

public final class CheckFloat {
    public static void main(String[] args) {
        String scenario = args[0];
        RuntimeException fault = new IllegalStateException("authored right fault");
        RuntimeException argument = scenario.endsWith("-fault") ? fault : null;
        FloatCompare.trace = 0;
        String outcome;
        try {
            outcome = switch (scenario) {
                case "finite-float-one" -> "return:" + FloatCompare.one();
                case "finite-float-zero" -> "return:" + FloatCompare.zero();
                case "finite-float-value" -> "return:" + FloatCompare.value();
                case "nan-float-high", "nan-float-high-fault" -> "return:" + FloatCompare.nanSkips(argument);
                case "nan-float-low", "nan-float-low-fault" -> "return:" + FloatCompare.lowFloatZero(argument);
                case "finite-double-one" -> "return:" + FloatCompare.doubleFinite();
                case "nan-double-high", "nan-double-high-fault" -> "return:" + FloatCompare.doubleHigh(argument);
                case "nan-double-low", "nan-double-low-fault" -> "return:" + FloatCompare.doubleLow(argument);
                default -> throw new AssertionError(scenario);
            };
        } catch (RuntimeException error) {
            outcome = "throw:" + (error == fault);
        }
        System.out.println(outcome + ",trace:" + FloatCompare.trace);
    }
}
