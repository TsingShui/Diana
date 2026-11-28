package origin;

public final class PrefixOwner {
    public static int value;

    static {
        outside.PrefixOwner.value = 7;
    }

    public static int observed() {
        return value;
    }
}
