package fixture;

public final class DexConsumer {
    public static String compileText() {
        return DexProvider.COMPILE_TEXT;
    }

    public static int compileNumber() {
        return DexProvider.COMPILE_NUMBER;
    }

    public static String runtimeText() {
        return DexProvider.RUNTIME_TEXT;
    }

    public static int runtimeNumber() {
        return DexProvider.RUNTIME_NUMBER;
    }
}
