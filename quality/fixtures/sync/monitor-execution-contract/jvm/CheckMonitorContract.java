import java.lang.reflect.Method;
import java.lang.reflect.Modifier;

public final class CheckMonitorContract {
    private static void check(String name, Object expected, Object observed) {
        System.out.println(name + "\t" + (expected.equals(observed) ? "PASS" : "FAIL")
                + "\t" + expected + "\t" + observed);
    }

    private static void flags(String name, boolean synchronizedMethod, boolean isNative, Class<?>... parameters) throws Exception {
        Method method = MonitorContract.class.getDeclaredMethod(name, parameters);
        int flags = method.getModifiers();
        check(name + "-declared", synchronizedMethod + ":" + isNative,
                Modifier.isSynchronized(flags) + ":" + Modifier.isNative(flags));
    }

    private static void fault(MonitorContract target, Object gate, boolean statik) {
        MonitorContract.faultHeld = -1;
        RuntimeException result = null;
        try {
            if (statik) MonitorContract.exceptionalStatic(gate);
            else target.exceptional(gate);
        } catch (RuntimeException error) {
            result = error;
        }
        String method = statik ? "exceptional-static" : "exceptional-instance";
        check(method + "-fault", true, result == MonitorContract.FAULT);
        check(method + "-held", 3, MonitorContract.faultHeld);
        check(method + "-released", "false:false", Thread.holdsLock(statik ? MonitorContract.class : target)
                + ":" + Thread.holdsLock(gate));
    }

    public static void main(String[] args) throws Exception {
        MonitorContract target = new MonitorContract();
        Object gate = new Object();
        check("different-active", 3, target.different(gate));
        check("different-released", "false:false", Thread.holdsLock(target) + ":" + Thread.holdsLock(gate));
        check("different-static-active", 3, MonitorContract.differentStatic(gate));
        check("different-static-released", "false:false", Thread.holdsLock(MonitorContract.class) + ":" + Thread.holdsLock(gate));
        check("same-reentry-held-after-inner", 3, target.sameReentry());
        check("same-reentry-released", false, Thread.holdsLock(target));
        fault(target, gate, false);
        fault(target, gate, true);
        check("implicit-instance-held", 1, target.implicit());
        check("implicit-static-held", 1, MonitorContract.implicitStatic());
        check("explicit-only-held", 2, target.explicitOnly(gate));
        MonitorContract.faultHeld = -1;
        target.nestedExplicit(gate);
        check("nested-explicit-held", 3, MonitorContract.faultHeld);
        check("nested-explicit-released", false, Thread.holdsLock(gate));
        flags("different", true, false, Object.class);
        flags("differentStatic", true, false, Object.class);
        flags("sameReentry", true, false);
        flags("exceptional", true, false, Object.class);
        flags("exceptionalStatic", true, false, Object.class);
        flags("implicit", true, false);
        flags("implicitStatic", true, false);
        flags("explicitOnly", false, false, Object.class);
        flags("nativeInstance", true, true);
        flags("nativeStatic", true, true);
        flags("nestedExplicit", false, false, Object.class);
    }
}
