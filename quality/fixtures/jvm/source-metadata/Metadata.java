package fixture.jvm;

import java.util.List;

public abstract class Metadata<T extends Number> {
    public List<T> history;

    public abstract T declared(T source);

    public <U extends T> U pick(long window, U candidate) {
        return candidate;
    }

    public double delta(long ticks, double amount) {
        return amount - ticks;
    }
}
