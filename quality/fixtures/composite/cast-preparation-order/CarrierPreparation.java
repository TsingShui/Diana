package fixture;

public class CarrierPreparation {
    static native Object produce();
    static native int consume(Object value);
    static int produced() { return consume((int[]) produce()); }
    static int castFirst(Object receiver, int flag) {
        if (flag != 0) return 1;
        int[] checked = (int[]) receiver;
        String argument = "x";
        return checked.equals(argument) ? 1 : 0;
    }
    static int ordered(Object receiver, int flag) {
        if (flag != 0) return 1;
        String argument = "x";
        int[] checked = (int[]) receiver;
        return checked.equals(argument) ? 1 : 0;
    }
}
