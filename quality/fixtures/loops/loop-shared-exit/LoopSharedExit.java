package fixture;

/** Atomic fixture: nested loops that leave through a flag or a labelled break. */
public final class LoopSharedExit {
    public static int flagExit(int[][] matrix, int target) {
        boolean found = false;
        int result = -1;
        for (int row = 0; row < matrix.length && !found; row++) {
            for (int column = 0; column < matrix[row].length; column++) {
                if (matrix[row][column] == target) {
                    found = true;
                    result = row * 100 + column;
                    break;
                }
            }
        }
        return result;
    }

    public static int labelledExit(int[][] matrix, int target) {
        outer:
        for (int row = 0; row < matrix.length; row++) {
            for (int column = 0; column < matrix[row].length; column++) {
                if (matrix[row][column] == target) {
                    return row * 100 + column;
                }
                if (matrix[row][column] < 0) {
                    break outer;
                }
            }
        }
        return -1;
    }

    public static int sharedTail(int[] values) {
        int total = 0;
        for (int index = 0; index < values.length; index++) {
            if (values[index] < 0) {
                continue;
            }
            total += values[index];
        }
        for (int index = 0; index < values.length; index++) {
            if (values[index] > 0) {
                continue;
            }
            total -= values[index];
        }
        return total;
    }
}
