package fixture;

public final class FragmentOrderLoops {
    public static int nestedSwitch(int flag, int start) {
        int n = start;
        if (flag == 0) return 7;
        while (n < 4) {
            switch (n) {
                case 0:
                case 1:
                    n++;
                    break;
                case 2:
                    n += 2;
                    break;
                default:
                    n++;
                    break;
            }
        }
        return n;
    }
}
