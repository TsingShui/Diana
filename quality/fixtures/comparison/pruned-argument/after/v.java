package y;

public class v {
    public static int r(int value) {
        int adjusted = Integer.reverse(value) ^ 37;
        return adjusted + Integer.bitCount(adjusted);
    }
}
