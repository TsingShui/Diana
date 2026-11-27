package fixture.record;

public record RecordProbe(int count, float reading, String label, int[] values) {
    public int incremented() {
        return count + 1;
    }
}
