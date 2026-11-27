package y;

public class t {
    public static int b(int x) {
        return x ^ 7;
    }

    public static int route(int x) {
        return b(Integer.lowestOneBit(x));
    }
}
