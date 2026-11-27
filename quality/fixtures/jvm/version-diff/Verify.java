import fixture.diff.Bootstrap;
import java.io.IOException;
import java.lang.reflect.Method;
import java.nio.file.Files;
import java.nio.file.Path;

public final class Verify {
    private final Path directory;

    Verify(Path directory) {
        this.directory = directory;
    }

    private static final class Loader extends ClassLoader {
        Class<?> loadBytes(byte[] bytes) {
            return defineClass(null, bytes, 0, bytes.length);
        }
    }

    private Class<?> load(String file) throws IOException {
        return new Loader().loadBytes(Files.readAllBytes(directory.resolve(file)));
    }

    static void expect(Object actual, Object expected) {
        if (!expected.equals(actual)) {
            throw new AssertionError("expected " + expected + ", got " + actual);
        }
    }

    private void methods(String file, boolean changed) throws Exception {
        Class<?> unit = load(file);
        expect(unit.getMethod("same").invoke(null), "shared\0\uD83C\uDF31");
        expect(unit.getMethod("message").invoke(null), changed ? "after" : "before");
        Method branch = unit.getMethod("branch", int.class);
        expect(branch.invoke(null, -9), 9);
        expect(branch.invoke(null, 0), changed ? 0 : -1);
        Method route = unit.getMethod("route", int.class);
        expect(route.invoke(null, -7), 0);
        expect(route.invoke(null, 6), 1);
        expect(route.invoke(null, 23), 3);
        expect(route.invoke(null, 100), changed ? 3 : 2);
        expect(unit.getMethod("caught").invoke(null), changed ? 2 : 1);
        expect(unit.getMethod("partial").invoke(null), changed ? 5 : 4);
        Method generic = unit.getMethod("generic");
        expect(generic.getGenericReturnType().getTypeName(),
               changed ? "java.util.List<java.lang.Integer>" : "java.util.List<java.lang.String>");
        expect(generic.getExceptionTypes()[0],
               changed ? IllegalStateException.class : IOException.class);
    }

    private void linkage(String file, boolean changed) throws Exception {
        Bootstrap.calls = 0;
        Class<?> unit = load(file);
        expect(unit.getMethod("argument").invoke(null), changed ? "new argument" : "old argument");
        expect(Bootstrap.calls, 1);
        expect(unit.getMethod("sharing").invoke(null), changed ? "new argument" : "old argument");
        expect(Bootstrap.calls, changed ? 2 : 1);
    }

    public static void main(String[] args) throws Exception {
        Verify verify = new Verify(Path.of(args[0]));
        verify.methods("old.class", false);
        verify.methods("width.class", false);
        verify.methods("changed.class", true);
        verify.linkage("linkage-old.class", false);
        verify.linkage("linkage-width.class", false);
        verify.linkage("linkage-changed.class", true);
    }
}
