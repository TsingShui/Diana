package fixture;

public final class CheckMonitor {
    public static void main(String[] args) {
        String scenario = args[0];
        MonitorContract receiver = new MonitorContract();
        Object lock = scenario.startsWith("named") ? new Object() : receiver;
        MonitorHooks.events.add("before:" + Thread.holdsLock(lock));
        try {
            String observed;
            switch (scenario) {
                case "metadata-only":
                    observed = Boolean.toString(receiver.metadataOnly());
                    break;
                case "physical":
                    observed = Boolean.toString(receiver.physical(false));
                    break;
                case "physical-fault":
                    observed = Boolean.toString(receiver.physical(true));
                    break;
                case "nested":
                    observed = Boolean.toString(receiver.nested(false));
                    break;
                case "nested-fault":
                    observed = Boolean.toString(receiver.nested(true));
                    break;
                case "named":
                    receiver.nestedNamed(lock, false);
                    observed = "void";
                    break;
                case "named-fault":
                    receiver.nestedNamed(lock, true);
                    observed = "void";
                    break;
                default:
                    throw new AssertionError(scenario);
            }
            MonitorHooks.events.add("return:" + observed);
        } catch (RuntimeException error) {
            MonitorHooks.events.add("throw:" + (error == MonitorHooks.FAULT));
        }
        MonitorHooks.events.add("after:" + Thread.holdsLock(lock));
        System.out.println(String.join(",", MonitorHooks.events));
    }
}
