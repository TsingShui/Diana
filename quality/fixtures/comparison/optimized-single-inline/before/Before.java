package bench;
public class Before {
  public static String marker() { return "same authored class"; }
  public static int run(int x, int divisor) {
    int rotated = Integer.rotateLeft(x, 3);
    int result = calc(rotated, divisor);
    return Integer.rotateRight(result, 7);
  }
  public static int calc(int x, int divisor) {
    int q = Integer.reverse(x);
    q = q / divisor;
    q = Math.abs(q);
    return q + 99;
  }
}
