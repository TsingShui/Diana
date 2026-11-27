package fixture;

public final class MutableGuardRoutes {
    public static int route(int seed, int fault) {
        return walk(seed, fault, 40, 2);
    }

    public static int extended(int seed, int fault) {
        return walk(seed, fault, 48, 5);
    }

    private static int walk(int seed, int fault, int end, int variant) {
        int node = 0;
        while (node < end) {
            boolean selected = GuardTrace.step(seed, fault, node) != 0;
            int group = node / 8;
            int base = group * 8;
            int next = base + 8;
            int later = Math.min(end, next + 8);
            node = switch (node % 8) {
                case 0 -> base + (selected ? 4 : 1);
                case 1 -> base + (selected ? 3 : 2);
                case 2 -> selected ? ((group + variant) % 2 == 0 ? base + 3 : next) : base + 7;
                case 3 -> selected ? ((group + variant) % 3 == 0 ? end : Math.min(end, next + 1)) : base + 7;
                case 4 -> base + (selected ? 6 : 5);
                case 5 -> selected ? Math.min(end, next + 4) : base + 7;
                case 6 -> selected ? (group % 2 == 0 ? end : later) : base + 7;
                default -> selected ? later : next;
            };
        }
        return end;
    }
}
