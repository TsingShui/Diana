public interface InterfaceSnapshot {
    Object RESULT = switch (0) {
        default -> {
            Object saved = SnapshotEffects.INSTANCE;
            SnapshotEffects.replace(saved, false);
            yield saved;
        }
    };
}
