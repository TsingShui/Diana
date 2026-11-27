package fixture.record;

public final class Verify {
    private static void check(boolean result) {
        if (!result) {
            throw new AssertionError("unexpected record result");
        }
    }

    public static void main(String[] args) {
        int[] values = {6, 9};
        RecordProbe original = new RecordProbe(7, Float.NaN, "marker", values);
        RecordProbe equal = new RecordProbe(7, Float.intBitsToFloat(0x7fc00001), "marker", values);
        check(original.equals(equal));
        check(original.hashCode() == equal.hashCode());
        check(!original.equals(new RecordProbe(7, Float.NaN, "marker", new int[] {6, 9})));
        check(!original.equals(null));
        check(!original.equals(new Empty()));
        check(original.incremented() == 8);

        RecordProbe zero = new RecordProbe(7, 0.0f, null, values);
        check(!zero.equals(new RecordProbe(7, -0.0f, null, values)));
        check(zero.toString().contains("count=7, reading=0.0, label=null, values="));
        check(new Empty().equals(new Empty()));
        check(new Empty().toString().equals("Empty[]"));
    }
}
