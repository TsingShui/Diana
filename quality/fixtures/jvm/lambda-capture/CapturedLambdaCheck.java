package fixture.jvm;

public final class CapturedLambdaCheck {
    public static void main(String[] args) {
        var first = CapturedLambda.captured(2, 4_294_967_296L + 9);
        var second = CapturedLambda.captured(7, 5L);
        if (CapturedLambda.trace() != 0) {
            throw new AssertionError("creating the lambdas must not execute their bodies");
        }
        if (second.applyAsInt(1) != 13 || CapturedLambda.trace() != 7) {
            throw new AssertionError("second capture must use its own immutable entries");
        }
        if (first.applyAsInt(-1) != 10 || CapturedLambda.trace() != 72) {
            throw new AssertionError("first capture must preserve both original words and its alias");
        }
        if (second.applyAsInt(-5) != 7 || CapturedLambda.trace() != 727) {
            throw new AssertionError("another application must reuse the same captured values");
        }
        var joinedFirst = CapturedLambda.joined(true, 2);
        var joinedSecond = CapturedLambda.joined(false, 9);
        if (CapturedLambda.trace() != 757 || joinedFirst.applyAsInt(5) != 7
                || joinedSecond.applyAsInt(-1) != 8) {
            throw new AssertionError("both joins must retain the entry capture and eager branch effects");
        }
    }
}
