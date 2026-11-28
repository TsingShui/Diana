import fixture.jvm.ExceptionChains;

public final class DecBenchHandlers {
    public static void main(String[] args) {
        int[] values = switch (args[1]) {
            case "quotient", "divide-zero" -> new int[]{21};
            case "bounds" -> new int[]{};
            case "null" -> null;
            default -> throw new IllegalArgumentException(args[1]);
        };
        boolean cleanup = switch (args[0]) {
            case "guarded" -> false;
            case "guardedFinally" -> true;
            default -> throw new IllegalArgumentException(args[0]);
        };
        int divisor = switch (args[1]) {
            case "divide-zero", "bounds" -> 0;
            default -> 3;
        };
        ExceptionChains input = new ExceptionChains();
        String kind;
        String outcome;
        try {
            int result = cleanup ? input.guardedFinally(values, divisor) : input.guarded(values, divisor);
            kind = "return";
            outcome = Integer.toString(result);
        } catch (Throwable failure) {
            kind = "throw";
            outcome = failure.getClass().getName();
        }
        System.out.println(kind + "\t" + outcome + "\t" + input.trace());
    }
}
