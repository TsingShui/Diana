package fixture;

public final class CrossWordBranches {
    // The Smali unrolls these 24 decisions.
    public static int run(int key, int value) {
        for (int bit = 0; bit < 24; bit++) {
            value = (key & 1) == 0
                ? ProtectedJoins.left(value)
                : ProtectedJoins.right(value);
            key >>>= 1;
        }
        return value;
    }
}
