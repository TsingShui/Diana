package fixture.jvm;

public final class UnsupportedCaptureCheck {
    public static void main(String[] args) {
        var computed = UnsupportedCapture.computed(5);
        if (UnsupportedCapture.trace() != 5 || computed.applyAsInt(1) != 6) {
            throw new AssertionError("the method call must occur at capture");
        }
        var overwritten = UnsupportedCapture.overwritten(7);
        if (UnsupportedCapture.trace() != 58 || overwritten.applyAsInt(1) != 8) {
            throw new AssertionError("the snapshot must use the original local value");
        }
        if (UnsupportedCapture.selected(true, 2, 5).applyAsInt(1) != 3
                || UnsupportedCapture.selected(false, 2, 5).applyAsInt(1) != 6) {
            throw new AssertionError("a join can select different values at capture");
        }
    }
}
