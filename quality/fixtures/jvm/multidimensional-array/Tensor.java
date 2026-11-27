package fixture.arrays;

public final class Tensor {
    private int order;

    private int dimension(int digit, int size) {
        order = order * 10 + digit;
        return size;
    }

    public String[][][] partial(int rows, int columns) {
        return new String[dimension(1, rows)][dimension(2, columns)][];
    }

    public int guarded(int rows, int columns, int depth) {
        order = 0;
        try {
            int[][][] values = new int[dimension(1, rows)][dimension(2, columns)][dimension(3, depth)];
            return values.length;
        } catch (NegativeArraySizeException error) {
            return -order;
        }
    }

    public int order() {
        return order;
    }
}
