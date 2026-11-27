package guard;

class Switches {
    static int dispatch(RuntimeException first, RuntimeException second, int key) {
        if (key == 17) throw second;
        try {
            if (key == -3) throw first;
            return 0;
        } catch (IllegalArgumentException ignored) {
            return -3;
        } catch (Throwable ignored) {
            return -5;
        }
    }

    static int preserved(RuntimeException first, RuntimeException second, int key) {
        try {
            if (key == -3) throw first;
            if (key == 17) throw second;
            return 0;
        } catch (IllegalArgumentException ignored) {
            return -3;
        } catch (Throwable ignored) {
            return -5;
        }
    }

}
