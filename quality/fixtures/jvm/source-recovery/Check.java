import fixture.jvm.SourceRecovery;

class Check {
    private static void require(boolean condition, String path) {
        if (!condition) {
            throw new AssertionError(path);
        }
    }

    public static void main(String[] args) {
        SourceRecovery input = new SourceRecovery(9);
        require(input.calculate(3) == 23, "constructor, field load, addition and helper");
        require(input.calculate(-10) == -3, "negative helper input");
        require(input.choose(4) == 13, "positive branch");
        require(input.choose(0) == 9, "boundary branch");
        require(input.choose(-3) == 12, "negative branch");
        require(SourceRecovery.descendingSum(5) == 15, "loop back edge");
        require(SourceRecovery.descendingSum(0) == 0, "loop skipped");
        require(SourceRecovery.descendingSum(-3) == 0, "negative loop input");
        require(SourceRecovery.divideOrDefault(21, 3) == 7, "normal division");
        require(SourceRecovery.divideOrDefault(21, 0) == -7, "typed exception handler");
        require(input.update(4) == 11, "field stores surrounding concatenation");
        require(input.last().equals("value=13"), "concatenation sees the first field store");
        require(input.choose(0) == 11, "later method sees the second field store");
        require(SourceRecovery.classify(-7) == 61, "first key on the shared switch path");
        require(SourceRecovery.classify(40) == 61, "second key on the shared switch path");
        require(SourceRecovery.classify(6) == -4, "separate switch path");
        require(SourceRecovery.classify(100) == 99, "switch path physically after the default");
        require(SourceRecovery.classify(41) == 241, "non-case switch input");
    }
}
