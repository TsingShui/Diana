package layout;

public class Route {
    public static int stable(int mode, int value) {
        if (mode <= 0) return value - 3;
        if (value == 0) return value - 3;
        return value * 3;
    }

    public static int redirect(int mode, int value) {
        if (mode <= 0) return value & 7;
        if (value == 0) return value ^ 13;
        return value & 7;
    }
}
