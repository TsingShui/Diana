public interface StableSnapshot {
    Object RESULT = switch (0) {
        default -> {
            Object saved = StableOwner.INSTANCE;
            SnapshotEffects.replace(saved, false);
            yield saved;
        }
    };
}
