public interface ProtectedSnapshot {
    Object RESULT = switch (0) {
        default -> {
            Object saved = SnapshotEffects.INSTANCE;
            try {
                SnapshotEffects.replace(saved, SnapshotEffects.throwOnReplace);
            } catch (IllegalStateException fault) {
                SnapshotEffects.record(saved, fault);
            }
            yield saved;
        }
    };
}
