package fixture;

public class PreparationParent {
    static {
        PreparationEvents.start();
        PreparationEvents.finish();
    }
}
