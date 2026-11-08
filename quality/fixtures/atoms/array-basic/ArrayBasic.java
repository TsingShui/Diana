package fixture;

/** Atomic fixture: array creation, filling, reads and stores. */
public final class ArrayBasic {
    public static int[] sized(int length) {
        return new int[length];
    }

    public static int[] filled() {
        return new int[] {1, 2, 3, 4};
    }

    public static byte[] bytes() {
        return new byte[] {1, 2, 3};
    }

    public static boolean[] flags() {
        return new boolean[] {true, false, true};
    }

    public static int read(int[] values, int index) {
        return values[index];
    }

    public static void write(int[] values, int index, int value) {
        values[index] = value;
    }

    public static int length(int[] values) {
        return values.length;
    }

    public static int nested(int[][] matrix, int row, int column) {
        return matrix[row][column];
    }
}
