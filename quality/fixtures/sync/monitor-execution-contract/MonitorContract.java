package fixture;

public final class MonitorContract {
    public boolean physical(boolean fail) {
        synchronized (this) {
            return MonitorHooks.probe(this, fail);
        }
    }

    public boolean metadataOnly() {
        return Thread.holdsLock(this);
    }

    public boolean nested(boolean fail) {
        synchronized (this) {
            synchronized (this) {
                return MonitorHooks.probe(this, fail);
            }
        }
    }

    public void nestedNamed(Object lock, boolean fail) {
        synchronized (lock) {
            synchronized (lock) {
                MonitorHooks.probeVoid(lock, fail);
            }
        }
    }

    public native synchronized void nativeSync();
}
