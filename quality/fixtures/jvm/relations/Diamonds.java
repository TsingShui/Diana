package fixture.relations;

public final class Diamonds {
    public static int read(Tip receiver) {
        return receiver.pick();
    }
}

interface Common {
    default int pick() {
        return 1;
    }
}

interface Left extends Common {}

interface Right extends Common {
    @Override
    default int pick() {
        return 2;
    }
}

interface Tip extends Left, Right {}

final class Winning implements Tip {}
