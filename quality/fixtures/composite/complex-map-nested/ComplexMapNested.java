package fixture;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** Combined fixture: nested generics, computeIfAbsent, getOrDefault, entry loops. */
public final class ComplexMapNested {
    private final Map<String, List<Integer>> groups = new LinkedHashMap<>();

    public void add(String key, int value) {
        groups.computeIfAbsent(key, name -> new ArrayList<>()).add(value);
    }

    public int total(String key) {
        List<Integer> values = groups.get(key);
        if (values == null) {
            return 0;
        }
        int total = 0;
        for (int value : values) {
            total += value;
        }
        return total;
    }

    public Map<String, Integer> totals() {
        Map<String, Integer> out = new LinkedHashMap<>();
        for (Map.Entry<String, List<Integer>> entry : groups.entrySet()) {
            int total = 0;
            for (int value : entry.getValue()) {
                total += value;
            }
            out.put(entry.getKey(), total);
        }
        return out;
    }

    public int countOrZero(String key) {
        return groups.getOrDefault(key, new ArrayList<>()).size();
    }

    public void merge(ComplexMapNested other) {
        for (Map.Entry<String, List<Integer>> entry : other.groups.entrySet()) {
            groups.computeIfAbsent(entry.getKey(), name -> new ArrayList<>()).addAll(entry.getValue());
        }
    }
}
