package fixture;

import java.util.Arrays;

/** Classic algorithms: insertion, quick and merge sort. */
public final class AlgoSort {
    public static void insertion(int[] values) {
        for (int index = 1; index < values.length; index++) {
            int key = values[index];
            int position = index - 1;
            while (position >= 0 && values[position] > key) {
                values[position + 1] = values[position];
                position--;
            }
            values[position + 1] = key;
        }
    }

    public static void quick(int[] values) {
        quick(values, 0, values.length - 1);
    }

    private static void quick(int[] values, int low, int high) {
        if (low >= high) {
            return;
        }
        int pivot = values[(low + high) / 2];
        int left = low;
        int right = high;
        while (left <= right) {
            while (values[left] < pivot) {
                left++;
            }
            while (values[right] > pivot) {
                right--;
            }
            if (left <= right) {
                int swap = values[left];
                values[left] = values[right];
                values[right] = swap;
                left++;
                right--;
            }
        }
        quick(values, low, right);
        quick(values, left, high);
    }

    public static int[] merge(int[] values) {
        if (values.length < 2) {
            return values.clone();
        }
        int middle = values.length / 2;
        int[] left = merge(Arrays.copyOfRange(values, 0, middle));
        int[] right = merge(Arrays.copyOfRange(values, middle, values.length));
        int[] merged = new int[values.length];
        int l = 0;
        int r = 0;
        int out = 0;
        while (l < left.length && r < right.length) {
            merged[out++] = left[l] <= right[r] ? left[l++] : right[r++];
        }
        while (l < left.length) {
            merged[out++] = left[l++];
        }
        while (r < right.length) {
            merged[out++] = right[r++];
        }
        return merged;
    }
}
