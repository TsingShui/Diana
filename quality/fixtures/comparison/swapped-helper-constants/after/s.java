package y;

public class s {
    public static int b(int x) {
        return x ^ 15;
    }

    public static int route(int x) {
        return b(Integer.reverseBytes(x));
    }
}
