final class WrongOrderAndScale {
    public static long parseTimeString(String input) {
        String[] components = input.split(":");
        if (components.length < 2) {
            return 0L;
        }
        boolean hasHours = components.length == 3;
        int minuteIndex = hasHours ? 1 : 0;
        long hourMillis = hasHours ? Integer.parseInt(components[0]) * 3_600_000L : 0L;
        long secondMillis = (long) Float.parseFloat(components[minuteIndex + 1]) * 1_000L;
        long minuteMillis = Integer.parseInt(components[minuteIndex]) * 60_000L;
        return hourMillis + minuteMillis + secondMillis;
    }
}
