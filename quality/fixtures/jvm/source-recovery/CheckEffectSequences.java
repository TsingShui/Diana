import fixture.jvm.EffectSequences;

final class CheckEffectSequences {
    private static void require(int observed, int expected) {
        if (observed != expected) {
            throw new AssertionError("expected " + expected + ", observed " + observed);
        }
    }

    public static void main(String[] args) {
        var sequence = new EffectSequences();
        require(sequence.exchange(7), 0);
        require(sequence.exchange(19), 7);
        require(sequence.accumulate(2), 1);
        require(sequence.updatePair(3, 5), 8);
    }
}
