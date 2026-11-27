package yy;

public class h {
    public static int marker(int x) {
        return Integer.bitCount(x);
    }

    public static int n(int x) {
        return ((x ^ 19) * 5) - 1;
    }
}
