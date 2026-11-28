public final class CheckSnapshot {
    public static void main(String[] args) {
        Object saved;
        switch (args[0]) {
            case "basic": saved = InterfaceSnapshot.RESULT; break;
            case "protected": saved = ProtectedSnapshot.RESULT; break;
            case "fault":
                SnapshotEffects.throwOnReplace = true;
                saved = ProtectedSnapshot.RESULT;
                break;
            case "stable": saved = StableSnapshot.RESULT; break;
            default: throw new AssertionError(args[0]);
        }
        System.out.printf("%b/%b/%d%n", saved == SnapshotEffects.BEFORE,
                SnapshotEffects.INSTANCE == SnapshotEffects.AFTER, SnapshotEffects.trace);
    }
}
