public interface ConditionalOperand {
    Object RESULT = switch (0) {
        default -> {
            Object saved = ScheduleEffects.capture();
            yield ScheduleEffects.flag() ? saved : ScheduleEffects.fallback();
        }
    };
}
