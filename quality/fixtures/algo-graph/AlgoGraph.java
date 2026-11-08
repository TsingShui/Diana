package fixture;

import java.util.ArrayDeque;
import java.util.Arrays;
import java.util.Deque;

/** Classic algorithms: BFS, DFS and connected components on an adjacency matrix. */
public final class AlgoGraph {
    public static int[] bfs(int[][] matrix, int source) {
        int[] distance = new int[matrix.length];
        Arrays.fill(distance, -1);
        Deque<Integer> queue = new ArrayDeque<>();
        distance[source] = 0;
        queue.add(source);
        while (!queue.isEmpty()) {
            int node = queue.remove();
            for (int next = 0; next < matrix.length; next++) {
                if (matrix[node][next] != 0 && distance[next] < 0) {
                    distance[next] = distance[node] + 1;
                    queue.add(next);
                }
            }
        }
        return distance;
    }

    public static int dfs(int[][] matrix, boolean[] visited, int node) {
        if (visited[node]) {
            return 0;
        }
        visited[node] = true;
        int count = 1;
        for (int next = 0; next < matrix.length; next++) {
            if (matrix[node][next] != 0) {
                count += dfs(matrix, visited, next);
            }
        }
        return count;
    }

    public static int components(int[][] matrix) {
        boolean[] visited = new boolean[matrix.length];
        int components = 0;
        for (int node = 0; node < matrix.length; node++) {
            if (!visited[node]) {
                dfs(matrix, visited, node);
                components++;
            }
        }
        return components;
    }
}
