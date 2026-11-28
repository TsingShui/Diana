package fixture;

public final class ForeignFieldAccess {
    public static int calls;

    public static int readRootThroughShadow(ShadowFields receiver) {
        return ((RootFields) receiver).value;
    }

    public static ShadowFields pick(ShadowFields receiver) {
        calls++;
        return receiver;
    }

    public static void writeChosenRoot(ShadowFields receiver, int next) {
        ((RootFields) pick(receiver)).value = next;
    }
}
