package edge;

public class Kernel {
    public static int mix(int a) {
        return Integer.reverseBytes(((a * 7) ^ 0x31) - 3);
    }
}
