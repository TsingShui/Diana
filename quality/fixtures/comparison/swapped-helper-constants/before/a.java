package x;

public class a {
    public static int a(int x) {
        return x ^ 7;
    }

    public static int route(int x) {
        return a(Integer.reverseBytes(x));
    }
}
