public final class MonitorContract {
    public static final RuntimeException FAULT = new IllegalStateException("original fault");
    public static int faultHeld;

    private static int held(Object implicitMonitor, Object explicitMonitor) {
        return (Thread.holdsLock(implicitMonitor) ? 1 : 0) + (Thread.holdsLock(explicitMonitor) ? 2 : 0);
    }

    public synchronized int different(Object gate) {
        synchronized (gate) {
            return held(this, gate);
        }
    }

    public static synchronized int differentStatic(Object gate) {
        synchronized (gate) {
            return held(MonitorContract.class, gate);
        }
    }

    public synchronized int sameReentry() {
        int nested;
        synchronized (this) {
            nested = Thread.holdsLock(this) ? 1 : 0;
        }
        return nested + (Thread.holdsLock(this) ? 2 : 0);
    }

    public synchronized void exceptional(Object gate) {
        synchronized (gate) {
            faultHeld = held(this, gate);
            throw FAULT;
        }
    }

    public static synchronized void exceptionalStatic(Object gate) {
        synchronized (gate) {
            faultHeld = held(MonitorContract.class, gate);
            throw FAULT;
        }
    }

    public synchronized int implicit() {
        return Thread.holdsLock(this) ? 1 : 0;
    }

    public static synchronized int implicitStatic() {
        return Thread.holdsLock(MonitorContract.class) ? 1 : 0;
    }

    public int explicitOnly(Object gate) {
        synchronized (gate) {
            return held(this, gate);
        }
    }

    public synchronized native int nativeInstance();

    public static synchronized native int nativeStatic();

    public void nestedExplicit(Object gate) {
        synchronized (gate) {
            synchronized (gate) {
                faultHeld = held(gate, gate);
            }
        }
    }
}
