package fixture;

/** Combined fixture: nested switch inside a loop inside try, with labelled breaks. */
public final class ComplexSwitchNested {
    private static int state;

    public static int run(int[] ops) {
        int applied = 0;
        try {
            outer:
            for (int index = 0; index < ops.length; index++) {
                switch (ops[index] % 4) {
                    case 0:
                        state = 1;
                        break;
                    case 1:
                        for (int inner = 0; inner < 3; inner++) {
                            if (inner == ops[index] % 3) {
                                continue outer;
                            }
                            applied += inner;
                        }
                        break;
                    case 2:
                        state = 2;
                        continue;
                    default:
                        break outer;
                }
                applied++;
            }
        } finally {
            state = state + applied;
        }
        return applied;
    }
}
