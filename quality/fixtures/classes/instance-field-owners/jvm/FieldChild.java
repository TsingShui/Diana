public class FieldChild extends FieldParent {
    public int slot = 5;

    public int readParent() {
        return super.slot;
    }

    public void writeParent(int value) {
        super.slot = value;
    }

    public int writeThenReadBoth(int value) {
        super.slot = value;
        this.slot += 2;
        return super.slot * 100 + this.slot;
    }

    @Override
    public int dispatch() {
        return 200 + this.slot;
    }

    public int callParent() {
        return super.dispatch();
    }
}
