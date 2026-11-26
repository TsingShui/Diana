package fixture;

public final class LargeArray {
    public static int[] partial(int value) {
        int[] array = new int[Integer.MAX_VALUE];
        array[0] = value;
        return array;
    }

    public static int[] complete(int first, int second) {
        return new int[]{ first, second };
    }

    public static int[] dependent(int value) {
        int[] array = new int[2];
        array[0] = value;
        array[1] = array[0];
        return array;
    }
}
