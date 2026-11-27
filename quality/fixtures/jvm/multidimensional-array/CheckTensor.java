package fixture.arrays;

public final class CheckTensor {
    public static void main(String[] args) {
        Tensor fixture = new Tensor();
        String[][][] partial = fixture.partial(2, 3);
        if (partial.length != 2 || partial[1].length != 3 || partial[0][0] != null || fixture.order() != 12) {
            throw new AssertionError("partial allocation");
        }
        if (fixture.guarded(2, 3, 4) != 2 || fixture.order() != 123) {
            throw new AssertionError("complete allocation");
        }
        if (fixture.guarded(0, 3, -1) != -123 || fixture.order() != 123) {
            throw new AssertionError("negative dimension after zero");
        }
        if (fixture.guarded(-1, 3, 4) != -123 || fixture.order() != 123) {
            throw new AssertionError("all dimension expressions precede the range check");
        }
    }
}
