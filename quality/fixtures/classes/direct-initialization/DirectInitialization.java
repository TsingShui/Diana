package fixture;

class Parent {
    int value;
    Parent(int value) { this.value = value; }
}

class Child extends Parent {
    Child(int value) { super(value); this.value = 99; }
}

public class DirectInitialization {
    // The Smali allocates Child and directly invokes Parent.<init>(value).
    // Java cannot express that call on a fresh Child. On normal completion
    // the object's value is the argument, not the Child constructor's 99.
    public static native Child direct(int value);

    public static Child ordinary(int value) { return new Child(value); }

    // Negative input guard: the Smali returns an uninitialized reference.
    public static native Child broken();
}
