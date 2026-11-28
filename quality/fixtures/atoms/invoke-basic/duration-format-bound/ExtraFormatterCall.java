final class ExtraFormatterCall {
    public static String getDurationString(long duration) {
        DateUtils.formatElapsedTime(duration);
        return DateUtils.formatElapsedTime(Math.max(duration, 0L));
    }
}
