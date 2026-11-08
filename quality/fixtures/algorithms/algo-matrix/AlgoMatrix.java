package fixture;

import java.util.ArrayList;
import java.util.List;

/** Classic algorithms: matrix multiply, transpose and spiral traversal. */
public final class AlgoMatrix {
    public static int[][] multiply(int[][] left, int[][] right) {
        int rows = left.length;
        int inner = right.length;
        int columns = right[0].length;
        int[][] out = new int[rows][columns];
        for (int row = 0; row < rows; row++) {
            for (int column = 0; column < columns; column++) {
                int sum = 0;
                for (int k = 0; k < inner; k++) {
                    sum += left[row][k] * right[k][column];
                }
                out[row][column] = sum;
            }
        }
        return out;
    }

    public static int[][] transpose(int[][] matrix) {
        int rows = matrix.length;
        int columns = matrix[0].length;
        int[][] out = new int[columns][rows];
        for (int row = 0; row < rows; row++) {
            for (int column = 0; column < columns; column++) {
                out[column][row] = matrix[row][column];
            }
        }
        return out;
    }

    public static List<Integer> spiral(int[][] matrix) {
        List<Integer> out = new ArrayList<>();
        int top = 0;
        int bottom = matrix.length - 1;
        int left = 0;
        int right = matrix[0].length - 1;
        while (top <= bottom && left <= right) {
            for (int column = left; column <= right; column++) {
                out.add(matrix[top][column]);
            }
            top++;
            for (int row = top; row <= bottom; row++) {
                out.add(matrix[row][right]);
            }
            right--;
            if (top <= bottom) {
                for (int column = right; column >= left; column--) {
                    out.add(matrix[bottom][column]);
                }
                bottom--;
            }
            if (left <= right) {
                for (int row = bottom; row >= top; row--) {
                    out.add(matrix[row][left]);
                }
                left++;
            }
        }
        return out;
    }
}
