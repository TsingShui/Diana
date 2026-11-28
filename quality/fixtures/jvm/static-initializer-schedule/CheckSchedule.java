public final class CheckSchedule {
    public static void main(String[] args) {
        ScheduleEffects.mode = Integer.parseInt(args[1]);
        String result;
        try {
            switch (args[0]) {
                case "earlier": result = identity(EarlierOperand.RESULT); break;
                case "conditional": result = identity(ConditionalOperand.RESULT); break;
                case "short": result = Boolean.toString(ShortOperand.RESULT); break;
                case "protected": result = identity(ProtectedOperand.RESULT); break;
                case "first": result = identity(FirstOperand.RESULT); break;
                default: throw new AssertionError(args[0]);
            }
        } catch (ExceptionInInitializerError failure) {
            Throwable cause = failure.getCause();
            if (cause == ScheduleEffects.CAPTURE_FAULT) {
                result = "capture-fault";
            } else if (cause == ScheduleEffects.EARLIER_FAULT) {
                result = "earlier-fault";
            } else if (cause == ScheduleEffects.AFTER_FAULT) {
                result = "after-fault";
            } else {
                result = "unexpected:" + cause;
            }
        }
        System.out.printf("%s/%d%n", result, ScheduleEffects.trace);
    }

    private static String identity(Object result) {
        if (result == ScheduleEffects.CAPTURED) return "captured";
        if (result == ScheduleEffects.OTHER) return "other";
        return "unexpected:" + result;
    }
}
