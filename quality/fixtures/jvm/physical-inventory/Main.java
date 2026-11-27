package fixture.jvm;

public class Main extends Base implements Marker {
    public static final String MESSAGE = "root\0🌱";
    public static final long FIXTURE_LONG = 0x100000001L;
    public static final double NEGATIVE_ZERO = -0.0;
    private int value;

    public Main() {
        value = 3;
    }

    public static int forward(int input) {
        if (input != 0) {
            return Helper.twice(input);
        }
        return 0;
    }

    public static long forward(long input) {
        return input;
    }

    public static String message() {
        return MESSAGE;
    }

    public int read() {
        return value;
    }
}

class Base {
    Base() {}
}

interface Marker {
    int read();
}

class Helper {
    static int twice(int value) {
        return value << 1;
    }
}

class Outer {
    static int increment(int input) {
        return input + 1;
    }
}

class Nested {
    static int probe(int input) {
        return Outer.increment(input);
    }
}
