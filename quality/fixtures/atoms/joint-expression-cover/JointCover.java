package fixture;

public class JointCover {
    static native int take(int x);
    static native int pair(int x, int y);
    static native void mark(int x);

    static int ordered(int flag) {
        if (flag != 0) return 7;
        int left = take(1);
        int right = take(2);
        int result = pair(left, right);
        return result != 0 ? 7 : 9;
    }

    static int blocked(int flag) {
        if (flag != 0) return 7;
        int left = take(1);
        int right = take(2);
        int result = pair(left, right);
        mark(result);
        return result != 0 ? 7 : 9;
    }

    static int ordinary(int seed) {
        int left = take(seed);
        int right = take(seed + 1);
        return pair(left, right);
    }

    static CoverBox constructed(int seed) {
        return new CoverBox(take(seed), take(seed + 1));
    }

    static int reversed(int seed) {
        int left = take(seed);
        int right = take(seed + 1);
        return pair(right, left);
    }
}

class CoverBox {
    int left, right;
    CoverBox(int left, int right) { this.left = left; this.right = right; }
}
