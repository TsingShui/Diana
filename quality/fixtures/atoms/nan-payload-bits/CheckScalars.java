package fixture;

public final class CheckScalars {
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
        ScalarNan.stored = 1.0d;
        ScalarNan.lastBits = 7;
        ScalarNan.caught = null;
        String observed;
        switch (args[0]) {
            case "floatValue":
                observed = bits(Float.floatToRawIntBits(ScalarNan.floatValue()));
                break;
            case "doubleValue":
                observed = bits(Double.doubleToRawLongBits(ScalarNan.doubleValue()));
                break;
            case "canonicalFloat":
                observed = bits(Float.floatToRawIntBits(ScalarNan.canonicalFloat()));
                break;
            case "bits":
                observed = bits(ScalarNan.bits(choice));
                break;
            case "store":
                ScalarNan.store();
                observed = bits(Double.doubleToRawLongBits(ScalarNan.stored));
                break;
            case "protectedBits":
                long returned = ScalarNan.protectedBits(choice, fault);
                observed = bits(returned) + ":" + bits(ScalarNan.lastBits) + ":"
                        + (ScalarNan.caught == null ? "normal"
                        : ScalarNan.caught == fault ? "same-fault" : "different-fault");
                break;
            default:
                throw new IllegalArgumentException(args[0]);
        }
        System.out.println(observed);
    }
}
