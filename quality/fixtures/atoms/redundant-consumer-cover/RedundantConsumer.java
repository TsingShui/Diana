package fixture;
final class Container { Container() { RedundantConsumer.made(); } }
public final class RedundantConsumer {
    static native void made();
    static native void after();
    static Container immediate() { Container value = new Container(); Container alias = value; return (Container) alias; }
    static Container barrier() { Container value = new Container(); Container alias = value; Container checked = (Container) alias; after(); return checked; }
}
