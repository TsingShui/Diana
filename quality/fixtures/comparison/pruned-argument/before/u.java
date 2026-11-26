package x;

public class u {
    public static int k(int value, int ignored) {
        int adjusted = Integer.reverse(value) ^ 31;
        return adjusted + Integer.bitCount(adjusted);
    }
}
