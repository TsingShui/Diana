package fixture;

public interface PublicationFace {
    Object FIRST = new Object();
    Object SECOND = initializeSecond(FIRST);

    static Object initializeSecond(Object saved) {
        observe();
        return saved;
    }

    static void observe() {
        HolderObservation.seen = FIRST;
    }
}

final class HolderObservation {
    static Object seen;
}

interface HolderTail {
    Object FIRST = new Object();
    Object SECOND = FIRST;
}
