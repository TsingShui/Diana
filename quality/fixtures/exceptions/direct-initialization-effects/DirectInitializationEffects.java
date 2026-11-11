package fixture;

public class DirectInitializationEffects {
    public static int trace;
    public static int fault;
    public static Child last;

    public static void event(int value) {
        trace = trace * 10 + value;
        if (fault == value) throw new IllegalStateException();
    }

    // Authoritative Smali: allocate Child, keep an alias, event(1),
    // event(3) only for a negative argument, invoke Parent.<init> on the
    // original object, event(4), publish the alias, and return it.
    // RuntimeException from any protected operation runs event(5), then
    // rethrows the same exception. If event(5) throws, that new error escapes.
    public static native Child build(int value);
}

class Parent {
    int value;
    Parent(int value) {
        DirectInitializationEffects.event(2);
        this.value = value;
    }
}

class Child extends Parent {
    Child(int value) { super(value); this.value = 99; }
}
