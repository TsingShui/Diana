package edge;

public class Alias {
    public static int a(int a) {
        return Integer.reverse(a) ^ 31;
    }

    public static int b(int a) {
        return Integer.bitCount(a) + 5;
    }
}
