package origin;

public final class LateOwner {
    public static int value;

    static {
        control.OwnerEvents.begin();
        outside.LateOwner.value = 5;
    }

    public static int observed() {
        return value;
    }
}
