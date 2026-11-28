class SwitchKey {
    final RuntimeException hashFailure = new IllegalStateException("hash");
    final RuntimeException equalFailure = new IllegalArgumentException("equals");
    int trace;

    private final String value;
    private final boolean accept;
    private final int fault;

    SwitchKey(String value, boolean accept, int fault) {
        this.value = value;
        this.accept = accept;
        this.fault = fault;
    }

    @Override public int hashCode() {
        trace = trace * 10 + 1;
        if (fault == 1) throw hashFailure;
        return value.hashCode();
    }

    @Override public boolean equals(Object label) {
        return observeEquals(2);
    }

    final boolean observeEquals(int event) {
        trace = trace * 10 + event;
        if (fault == 2) throw equalFailure;
        return accept;
    }
}

final class OverloadedSwitchKey extends SwitchKey {
    OverloadedSwitchKey(String value, boolean accept, int fault) {
        super(value, accept, fault);
    }

    public boolean equals(String label) {
        return observeEquals(label.equals("Aa") ? 3 : 4);
    }
}
