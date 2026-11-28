package fixture;

public final class CheckSignedSelections {
    private static String bits(int value) {
        return String.format(java.util.Locale.ROOT, "%08x", value);
    }

    private static String bits(long value) {
        return String.format(java.util.Locale.ROOT, "%016x", value);
    }

    public static void main(String[] args) {
        boolean choice = Boolean.parseBoolean(args[1]);
        ArithmeticException fault = Boolean.parseBoolean(args[2])
                ? new ArithmeticException("fixture fault") : null;
        SignedSelections.staticFloat = 1.0f;
        SignedSelections.staticDouble = 1.0d;
        SignedSelections.lastBits = 7;
        SignedSelections.caught = null;
        String observed;
        switch (args[0]) {
            case "rawFloat":
                observed = bits(SignedSelections.rawFloat(choice));
                break;
            case "rawDouble":
                observed = bits(SignedSelections.rawDouble(choice));
                break;
            case "storeFloat":
                SignedSelections.storeFloat(choice);
                observed = bits(Float.floatToRawIntBits(SignedSelections.staticFloat));
                break;
            case "storeDouble":
                SignedSelections.storeDouble(choice);
                observed = bits(Double.doubleToRawLongBits(SignedSelections.staticDouble));
                break;
            case "selectFloat":
                observed = bits(Float.floatToRawIntBits(SignedSelections.selectFloat(choice)));
                break;
            case "protectedUse":
                int returned = SignedSelections.protectedUse(choice, fault);
                observed = bits(returned) + ":" + bits(SignedSelections.lastBits)
                        + ":" + (SignedSelections.caught == null ? "normal"
                        : SignedSelections.caught == fault ? "same-fault" : "different-fault");
                break;
            default:
                throw new IllegalArgumentException(args[0]);
        }
        System.out.println(observed);
    }
}
