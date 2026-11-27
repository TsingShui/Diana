package guard;

class Conditional {
    static Object run(RuntimeException error, int key) {
        try {
            if (key == 0) throw error;
            return error;
        } catch (RuntimeException ignored) {
            return null;
        }
    }
}
