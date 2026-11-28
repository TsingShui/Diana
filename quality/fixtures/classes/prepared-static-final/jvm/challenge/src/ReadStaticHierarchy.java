public final class ReadStaticHierarchy {
    public static long inheritedLong() {
        return HierarchyAlias.PREPARED_LONG;
    }

    public static String directNull() {
        return DefaultHierarchyOwner.PREPARED_NULL;
    }

    public static int activeAlias() {
        return HierarchyAlias.ALIAS_VALUE;
    }

    public static long compiledLong() {
        return HierarchyAlias.COMPILED_LONG;
    }

    public static String compiledText() {
        return DefaultHierarchyOwner.COMPILED_TEXT;
    }
}
