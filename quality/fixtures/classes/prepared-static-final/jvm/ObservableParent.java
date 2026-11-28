public class ObservableParent {
    static {
        ParentEffects.start();
        ParentEffects.record("parent:complete");
    }
}
