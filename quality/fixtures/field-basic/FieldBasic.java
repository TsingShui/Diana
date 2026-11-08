package fixture;

/** Atomic fixture: static and instance field reads and writes. */
public final class FieldBasic {
    private static int counter;
    private static String label = "init";
    private int value;

    public static int readStatic() {
        return counter;
    }

    public static void writeStatic(int next) {
        counter = next;
    }

    public static String readLabel() {
        return label;
    }

    public int readInstance() {
        return value;
    }

    public void writeInstance(int next) {
        value = next;
    }

    public int addToInstance(int delta) {
        value += delta;
        return value;
    }
}
