package fixture;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.Iterator;
import java.util.List;
import java.util.Map;

/** Atomic fixture: map views and a custom Iterable behind an enhanced for. */
public final class LoopMapViews {
    static final class Names implements Iterable<String> {
        private final List<String> items = new ArrayList<>();

        void add(String item) {
            items.add(item);
        }

        @Override
        public Iterator<String> iterator() {
            return items.iterator();
        }
    }

    private final Map<String, Integer> counts = new HashMap<>();

    public void put(String key, int value) {
        counts.put(key, value);
    }

    public int sumValues() {
        int total = 0;
        for (int value : counts.values()) {
            total += value;
        }
        return total;
    }

    public List<String> keys() {
        List<String> out = new ArrayList<>();
        Iterator<String> iterator = counts.keySet().iterator();
        while (iterator.hasNext()) {
            out.add(iterator.next());
        }
        return out;
    }

    public int walk(Names names) {
        int total = 0;
        for (String name : names) {
            total += name.length();
        }
        return total;
    }

    public int drainValues() {
        int total = 0;
        for (Iterator<Integer> iterator = counts.values().iterator(); iterator.hasNext(); ) {
            total += iterator.next();
            iterator.remove();
        }
        return total;
    }
}
