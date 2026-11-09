package fixture;

public class CarrierContexts {
    static native Object make();
    static int triplet(Object a, Object b, Object c) {
        int[] checked = (int[]) b;
        return a == checked && checked == c ? 1 : 0;
    }
    static int caught(Object a, Object b) {
        try {
            int[] checked = (int[]) b;
            return a == checked ? 1 : 0;
        } catch (ClassCastException error) {
            return -1;
        }
    }
    static int acting(Object value) {
        int[] checked = (int[]) value;
        Object left = make();
        return left == checked ? 1 : 0;
    }
}
