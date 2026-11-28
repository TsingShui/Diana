package fixture;

public interface RuntimeFace {
    String FIRST = InterfaceEffects.literal();
    String SECOND = InterfaceEffects.second();

    static String readFirst() {
        return FIRST;
    }

    static String readSecond() {
        return SECOND;
    }
}
