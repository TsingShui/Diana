package fixture;

// Behavioral explanations; Smali defines the original control flow.
public final class FragmentOrder {
    public static int branchLoop(int flag) {
        int n = 0;
        if (flag == 0) return 7;
        while (n < 3) n++;
        return n;
    }

    // Smali enters either half of the same alternating cycle. This explanation
    // states the behavior without prescribing Diana's output structure.
    public static int irreducible(int flag) {
        int n = 0;
        if (flag == 0) {
            n++;
            if (n >= 4) return n;
        }
        while (true) {
            n += 2;
            if (n >= 4) return n;
            n++;
            if (n >= 4) return n;
        }
    }

    public static void selfCycle() {
        while (true) { }
    }
}
