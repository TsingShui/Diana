public final class ProtectedOperand {
    public static Object RESULT;

    static {
        Object saved = ScheduleEffects.capture();
        RESULT = ScheduleEffects.join(ScheduleEffects.earlier(), saved);
        try {
            ScheduleEffects.after(RESULT);
        } catch (IllegalStateException failure) {
            ScheduleEffects.record(failure);
        }
    }
}
