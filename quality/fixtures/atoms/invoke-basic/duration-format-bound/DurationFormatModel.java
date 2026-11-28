import java.util.ArrayList;
import java.util.List;

public final class DurationFormatModel {
    private static final List<String> EVENTS = new ArrayList<>();

    private static final class DateUtils {
        static String formatElapsedTime(long seconds) {
            EVENTS.add("format:" + seconds);
            return "elapsed:" + seconds;
        }
    }

    /* DECBENCH_SELECTED_METHOD */

    public static void main(String[] args) {
        String result = getDurationString(Long.parseLong(args[0]));
        EVENTS.add("return:" + result);
        System.out.println(String.join(",", EVENTS));
    }
}
