package bench;
public class After {
  public static String tag() { return "same authored class"; }
  public static int dispatch(int x, int divisor) {
    int rotated = Integer.rotateLeft(x, 3);
    int result = step(rotated, divisor);
    return Integer.rotateRight(result, 7);
  }
  static int step(int x, int divisor) {
    int q = Integer.reverse(x);
    q = q / divisor;
    q = Math.abs(q);
    return q + 99;
  }
}
