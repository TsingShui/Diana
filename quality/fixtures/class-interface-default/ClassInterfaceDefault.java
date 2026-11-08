package fixture;

/** Atomic fixture: interface default and static methods. */
public interface ClassInterfaceDefault {
    int value();

    default int doubled() {
        return value() * 2;
    }

    default int plus(int extra) {
        if (extra < 0) {
            return value();
        }
        return value() + extra;
    }

    static int constant() {
        return 7;
    }
}
