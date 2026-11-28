package fixture.jvm;

/** Authored witnesses for field effect order and local values passed between effects. */
public final class EffectSequences {
    private int left;
    private int right;
    private int count;

    public int exchange(int next) {
        int previous = left;
        left = next;
        right = previous;
        return right;
    }

    public int accumulate(int delta) {
        left += delta;
        int current = left;
        count++;
        right = current;
        return count;
    }

    public int updatePair(int first, int second) {
        left = first;
        right = second;
        return first + second;
    }
}
