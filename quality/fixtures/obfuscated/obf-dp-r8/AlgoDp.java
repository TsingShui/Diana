package fixture;

/** Classic dynamic programming: edit distance, longest increasing subsequence, knapsack. */
public final class AlgoDp {
    public static int editDistance(String left, String right) {
        int[][] cost = new int[left.length() + 1][right.length() + 1];
        for (int i = 0; i <= left.length(); i++) {
            cost[i][0] = i;
        }
        for (int j = 0; j <= right.length(); j++) {
            cost[0][j] = j;
        }
        for (int i = 1; i <= left.length(); i++) {
            for (int j = 1; j <= right.length(); j++) {
                if (left.charAt(i - 1) == right.charAt(j - 1)) {
                    cost[i][j] = cost[i - 1][j - 1];
                } else {
                    int replace = cost[i - 1][j - 1];
                    int delete = cost[i - 1][j];
                    int insert = cost[i][j - 1];
                    cost[i][j] = 1 + Math.min(replace, Math.min(delete, insert));
                }
            }
        }
        return cost[left.length()][right.length()];
    }

    public static int longestIncreasing(int[] values) {
        if (values.length == 0) {
            return 0;
        }
        int[] best = new int[values.length];
        int longest = 1;
        for (int i = 0; i < values.length; i++) {
            best[i] = 1;
            for (int j = 0; j < i; j++) {
                if (values[j] < values[i] && best[j] + 1 > best[i]) {
                    best[i] = best[j] + 1;
                }
            }
            if (best[i] > longest) {
                longest = best[i];
            }
        }
        return longest;
    }

    public static int knapsack(int[] weights, int[] values, int capacity) {
        int[] best = new int[capacity + 1];
        for (int item = 0; item < weights.length; item++) {
            for (int room = capacity; room >= weights[item]; room--) {
                int candidate = best[room - weights[item]] + values[item];
                if (candidate > best[room]) {
                    best[room] = candidate;
                }
            }
        }
        return best[capacity];
    }
}
