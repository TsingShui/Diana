package x;

public class b {
    public static int a(int x) {
        return x ^ 15;
    }

    public static int route(int x) {
        return a(Integer.lowestOneBit(x));
    }
}
