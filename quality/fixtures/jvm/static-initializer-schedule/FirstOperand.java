public final class FirstOperand {
    public static Object RESULT;

    static {
        Object saved = ScheduleEffects.capture();
        RESULT = ScheduleEffects.joinFirst(saved, ScheduleEffects.earlier());
    }
}
