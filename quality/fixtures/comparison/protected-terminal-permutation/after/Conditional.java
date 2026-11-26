package guard;

class Conditional {
    static Object run(RuntimeException error, int key) {
        if (key == 0) throw error;
        return error;
    }
}
