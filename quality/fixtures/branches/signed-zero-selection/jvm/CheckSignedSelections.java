import java.util.Locale;

public final class CheckSignedSelections {
    private static String bits(int value) { return String.format(Locale.ROOT, "%08x", value); }
    private static String bits(long value) { return String.format(Locale.ROOT, "%016x", value); }
    private static String raw(float value) { return bits(Float.floatToRawIntBits(value)); }
    private static String raw(double value) { return bits(Double.doubleToRawLongBits(value)); }

    private static void check(String name, String expected, String observed) {
        System.out.println(name + "\t" + expected + "\t" + observed);
    }

    public static void main(String[] args) throws ReflectiveOperationException {
        for (boolean first : new boolean[]{true, false}) {
            String f = first ? "80000000" : "00000000";
            String d = first ? "0000000000000000" : "8000000000000000";
            check("direct-float-" + first, f, raw(SignedSelections.selectFloat(first)));
            check("direct-double-" + first, d, raw(SignedSelections.selectDouble(first)));
            check("raw-float-" + first, f, bits(SignedSelections.rawFloat(first)));
            check("raw-double-" + first, d, bits(SignedSelections.rawDouble(first)));
            SignedSelections.store(first);
            check("static-store-" + first, f + "," + d,
                    raw(SignedSelections.staticFloat) + "," + raw(SignedSelections.staticDouble));
            SignedSelections.trace = 0;
            check("effectful-" + first, f + ",1",
                    bits(SignedSelections.effectful(first)) + "," + SignedSelections.trace);
            check("same-sign-" + first, "8000000000000000", raw(SignedSelections.sameSign(first)));
            String arrayFirstBits = raw(17.0f);
            float[] array = {17.0f, 19.0f};
            String value = bits(SignedSelections.guardedStore(array, 1, first));
            check("array-store-" + first, f + "," + arrayFirstBits + "," + f + ",12",
                    value + "," + raw(array[0]) + "," + raw(array[1]) + "," + SignedSelections.trace);
            float[] faultArray = {17.0f, 19.0f};
            int bounds = SignedSelections.guardedStore(faultArray, -1, first);
            check("array-bounds-" + first, "301,41880000,41980000,13",
                    bounds + "," + raw(faultArray[0]) + "," + raw(faultArray[1]) + "," + SignedSelections.trace);
            int absent = SignedSelections.guardedStore(null, 0, first);
            check("array-null-" + first, "302,14", absent + "," + SignedSelections.trace);
        }
        check("nested-false-false", "8000000000000000", bits(SignedSelections.nestedDouble(false, false)));
        check("nested-false-true", "8000000000000000", bits(SignedSelections.nestedDouble(false, true)));
        check("nested-true-false", "0000000000000000", bits(SignedSelections.nestedDouble(true, false)));
        check("nested-true-true", "8000000000000000", bits(SignedSelections.nestedDouble(true, true)));
        Class<?> type = SignedSelections.class;
        check("encoded-fields", "80000000,00000000,8000000000000000,0000000000000000",
                raw(type.getField("FIELD_NEG_FLOAT").getFloat(null)) + ","
                + raw(type.getField("FIELD_POS_FLOAT").getFloat(null)) + ","
                + raw(type.getField("FIELD_NEG_DOUBLE").getDouble(null)) + ","
                + raw(type.getField("FIELD_POS_DOUBLE").getDouble(null)));
    }
}
