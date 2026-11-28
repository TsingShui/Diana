package control;

public final class OwnerEvents {
    static int calls;
    static int before;

    public static void begin() {
        calls++;
        before = outside.LateOwner.value;
    }
}
