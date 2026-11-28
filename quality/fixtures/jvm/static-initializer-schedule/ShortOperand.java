public final class ShortOperand {
    public static boolean RESULT;

    static {
        boolean saved = ScheduleEffects.captureBoolean();
        RESULT = ScheduleEffects.flag() && saved;
    }
}
