package fixture;

public class CarrierOrder {
    static native Object make();
    static native void sink(int n, Object value);
    static int right(Object left, Object right) {
        int[] checked = (int[]) right;
        return left == checked ? 1 : 0;
    }
    static int left(Object left, Object right) {
        int[] checked = (int[]) left;
        return checked == right ? 1 : 0;
    }
    static int guarded(Object left, Object right, int flag) {
        int[] checked = (int[]) right;
        if (flag != 0) return 99;
        return left == checked ? 1 : 0;
    }
    static Object afterEffect(Object value) {
        int[] checked = (int[]) value;
        make();
        return checked;
    }
    static int arrayLength(Object value) {
        int[] checked = (int[]) value;
        return checked.length;
    }
    static void argument(Object value) {
        int[] checked = (int[]) value;
        sink(0, checked);
    }
}
