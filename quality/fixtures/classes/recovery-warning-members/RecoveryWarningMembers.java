package fixture;

/**
 * Explanations only: the Smali is authoritative. The failed bodies deliberately
 * expose uninitialized references, which ordinary Java cannot express.
 */
public class RecoveryWarningMembers {
    public static native Object broken();

    public static void originalThrow() {
        throw new UnsupportedOperationException(
            "Method could not be decompiled during semantic recovery: original application message");
    }

    public static int sibling() {
        return 7;
    }
}

class BrokenConstructor {
    public static Object leaked;

    public BrokenConstructor() {
        // Smali allocates Object, publishes that uninitialized reference to
        // leaked, then returns. No Object constructor is invoked on it.
    }

    public static int sibling() {
        return 9;
    }
}

class BrokenInitializer {
    public static Object leaked;

    static {
        // Smali allocates Object, publishes that uninitialized reference to
        // leaked, then returns. No Object constructor is invoked on it.
    }

    public static int sibling() {
        return 11;
    }
}
