package edge;

public class Gate {
    public static int evaluate(int a, int[] divisors, int index) {
        int mixed = Kernel.mix(a);
        try {
            return mixed / divisors[index];
        } catch (ArithmeticException failure) {
            return -1;
        }
    }
}
