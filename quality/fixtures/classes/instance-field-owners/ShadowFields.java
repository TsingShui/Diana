package fixture;

public class ShadowFields extends ParentFields {
    public int value;

    public int readOwn() {
        return this.value;
    }

    public int readParent() {
        return super.value;
    }

    public int readRoot() {
        return ((RootFields) this).value;
    }

    public void writeParent(int next) {
        super.value = next;
    }

    public void writeRoot(int next) {
        ((RootFields) this).value = next;
    }

    public int writePeerRoot(ShadowFields peer, int next) {
        ((RootFields) peer).value = next;
        return ((RootFields) this).value;
    }
}
