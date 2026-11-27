package fixture.relations;

public final class Relations {
    public static final int CODE = 9;

    public static int outer(int value) {
        return Terminal.bridge(value);
    }

    public static int virtualDispatch(Leaf receiver, int value) {
        return receiver.transform(value);
    }

    public static int interfaceDispatch(Child receiver, int value) {
        return receiver.send(value);
    }

    public static int inherited(int value) {
        return Leaf.step(value);
    }
}

interface Signal {
    default int send(int value) {
        return value + 1;
    }
}

interface Child extends Signal {}

class Base {
    protected int transform(int value) {
        return value + 10;
    }

    static int step(int value) {
        return value - 2;
    }
}

class Middle extends Base implements Child {
    @Override
    protected int transform(int value) {
        return value * 2;
    }
}

class Leaf extends Middle {
    @Override
    public int send(int value) {
        return value - 1;
    }

    @Override
    protected int transform(int value) {
        return super.transform(value) + 3;
    }
}

class Terminal {
    static int seed(int value) {
        return value + 7;
    }

    static int self(int depth) {
        return depth <= 0 ? seed(depth) : self(depth - 1);
    }

    static int bridge(int value) {
        return self(value) + self(value + 1);
    }
}
