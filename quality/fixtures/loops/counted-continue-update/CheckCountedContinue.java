package fixture;

import java.util.Arrays;

final class CheckCountedContinue {
    public static void main(String[] args) {
        int[] values = switch (args[1]) {
            case "null" -> null;
            case "" -> new int[0];
            default -> Arrays.stream(args[1].split(",")).mapToInt(Integer::parseInt).toArray();
        };
        int limit = Integer.parseInt(args[2]);
        ArithmeticException fault = Boolean.parseBoolean(args[3]) ? new ArithmeticException("fixture") : null;
        CountedContinueUpdate.state = 17;
        CountedContinueUpdate.events = 3;
        CountedContinueUpdate.lastValue = -99;
        String outcome;
        try {
            int result = switch (args[0]) {
                case "advanceThenStop" -> CountedContinueUpdate.advanceThenStop(values, limit, fault);
                case "holdOnZero" -> CountedContinueUpdate.holdOnZero(values);
                case "observeAfterAdvance" -> CountedContinueUpdate.observeAfterAdvance(values, fault);
                default -> throw new AssertionError(args[0]);
            };
            outcome = "return:" + result;
        } catch (NullPointerException exception) {
            outcome = "null-array-fault";
        } catch (ArithmeticException exception) {
            outcome = exception == fault ? "supplied-fault" : "different-fault";
        }
        System.out.println("[\"" + outcome + "\"," + CountedContinueUpdate.state
            + "," + CountedContinueUpdate.events + "," + CountedContinueUpdate.lastValue
            + "," + Arrays.toString(values) + "]");
    }
}
