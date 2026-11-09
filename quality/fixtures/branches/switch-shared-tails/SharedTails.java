package fixture;

public final class SharedTails {
    public static int trace;
    public static void mark(int event) { trace = trace * 31 + event; }
    public static int read(int key, int mode) {
        mark(100 + key);
        if (mode == 1 && key % 10 == 0) throw new IllegalArgumentException("read");
        if ((mode == 2 || mode == 4) && key == 1) throw new AssertionError("read");
        return key + 1;
    }
    public static void cleanup(int mode) {
        mark(70);
        if (mode == 3 || mode == 4) throw new IllegalStateException("cleanup");
    }
    public static int route(int rows, int key, int cut, int bonus, int mode) {
        int row = 0, sum = 0;
        try {
            outer: while (row < rows) {
                int carry = bonus;
                // Smali fixes shared tail A (+3/carry=sum), then B. The Java
                // explanation uses this flag without adding a DEX dispatcher.
                boolean throughA;
                switch (key) {
                    case 0:
                        mark(10 + row); sum += 1; throughA = true; break;
                    case 1:
                        int inner = 0;
                        throughA = true;
                        while (inner < 2) {
                            try { sum += read(row * 10 + inner, mode); }
                            catch (IllegalArgumentException failure) { mark(30 + inner); sum += 5; }
                            if (inner == cut) { carry = sum; throughA = false; break; }
                            inner += 1;
                        }
                        break;
                    case 2:
                        mark(20 + row); carry = bonus + row; throughA = false; break;
                    case 3:
                        throughA = false; break;
                    default:
                        mark(90 + row); break outer;
                }
                if (throughA) { sum += 3; carry = sum; }
                sum = carry;
                mark(50 + row);
                row += 1;
            }
        } finally { cleanup(mode); }
        return sum;
    }
    public static int conditional(int key, int flag, int a, int b) {
        int first = a, second = b;
        switch (key) {
            case 0:
                if (flag == 0) { mark(61); return first - second; }
                break;
            case 1: first = a + 1; second = b + 2; break;
            case 2: first = a + 7; second = b + 8; mark(61); return first - second;
            default: return -9;
        }
        // Smali has one shared A and one shared B, not duplicated mark calls.
        mark(60); first += second;
        mark(61); return first - second;
    }
    public static int foreign(int key, int flag) {
        // Smali jumps from the pre-selector branch to the ONE original case0
        // block; normalizing the pure selector here explains equivalent paths.
        if (flag != 0) key = 0;
        int sum = 0;
        switch (key) {
            case 0: mark(10); sum += 1; break;
            case 1: mark(20); sum += 2; break;
            case 2: mark(30); return sum + 3 + 7;
            default: return -1;
        }
        sum += 3;
        sum += 7;
        return sum;
    }
}
