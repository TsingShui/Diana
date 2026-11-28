public final class FieldPeer {
    public static int evaluations;

    private static FieldChild observed(FieldChild receiver) {
        evaluations += 1;
        return receiver;
    }

    public static int readPeer(FieldChild receiver) {
        return ((FieldParent) observed(receiver)).slot;
    }

    public static void writePeer(FieldChild receiver, int value) {
        ((FieldParent) observed(receiver)).slot = value;
    }
}
