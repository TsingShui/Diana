public final class EarlierOperand {
    public static Object RESULT;

    static {
        Object saved = ScheduleEffects.capture();
        RESULT = ScheduleEffects.join(ScheduleEffects.earlier(), saved);
    }
}
