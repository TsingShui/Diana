package edge;

public class Gate {
    public static int evaluate(int a, int[] divisors, int index) {
        int mixed = Integer.reverseBytes(((a * 7) ^ 0x31) - 3);
        try {
            return mixed / divisors[index];
        } catch (RuntimeException failure) {
            return -1;
        }
    }
}
