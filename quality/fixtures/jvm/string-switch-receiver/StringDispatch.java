public final class StringDispatch {
    public static int custom(SwitchKey key) {
        int selected = -1;
        switch (key.hashCode()) {
            case 97:
                if (key.equals("a")) {
                    selected = 0;
                    break;
                }
                selected = -1;
                break;
            default:
                selected = -1;
                break;
        }
        switch (selected) {
            case 0: return 100;
            default: return 300;
        }
    }

    public static int overloaded(OverloadedSwitchKey key) {
        int selected = -1;
        switch (key.hashCode()) {
            case 122:
                if (key.equals("z")) {
                    selected = 0;
                    break;
                }
                selected = -1;
                break;
            case 2112:
                if (key.equals("Aa")) {
                    selected = 1;
                    break;
                }
                selected = -1;
                break;
            default:
                selected = -1;
                break;
        }
        switch (selected) {
            case 0: return 100;
            case 1: return 200;
            default: return 300;
        }
    }

    public static int genuine(String key) {
        switch (key) {
            case "a": return 100;
            case "b": return 200;
            default: return 300;
        }
    }

    public static int genuineCollision(String key) {
        switch (key) {
            case "Aa": return 100;
            case "BB": return 200;
            default: return 300;
        }
    }
}
