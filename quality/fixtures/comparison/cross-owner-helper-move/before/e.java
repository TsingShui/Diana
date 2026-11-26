package xx;

public class e {
    public static int marker(int x) {
        return Integer.reverseBytes(x);
    }

    public static int read(int x) {
        return Integer.highestOneBit(e.m(x));
    }

    public static int write(int x) {
        return Integer.numberOfLeadingZeros(e.m(x));
    }

    public static int m(int x) {
        return ((x ^ 17) * 5) - 1;
    }
}
