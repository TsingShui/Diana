package fixture;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.Iterator;
import java.util.List;
import java.util.Map;

/** Combined fixture: iterator loops, enhanced for, map entries and removal. */
public final class ComplexIteration {
    private final List<String> names = new ArrayList<>();
    private final Map<String, Integer> counts = new HashMap<>();

    public void add(String name, int weight) {
        names.add(name);
        counts.put(name, weight);
    }

    public int totalWeight() {
        int total = 0;
        for (Map.Entry<String, Integer> entry : counts.entrySet()) {
            total += entry.getValue();
        }
        return total;
    }

    public void dropShort(int minimum) {
        Iterator<String> iterator = names.iterator();
        while (iterator.hasNext()) {
            String name = iterator.next();
            if (name.length() < minimum) {
                iterator.remove();
            }
        }
    }

    public List<String> heavy(int threshold) {
        List<String> out = new ArrayList<>();
        for (String name : names) {
            Integer weight = counts.get(name);
            if (weight != null && weight > threshold) {
                out.add(name);
            }
        }
        return out;
    }

    public int sumNested(int[][] matrix) {
        int total = 0;
        for (int[] row : matrix) {
            for (int value : row) {
                total += value;
            }
        }
        return total;
    }
}
