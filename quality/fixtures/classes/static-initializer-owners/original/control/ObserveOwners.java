package control;

public final class ObserveOwners {
    public static void main(String[] args) {
        if (args[0].equals("prefix")) {
            int observed = origin.PrefixOwner.observed();
            System.out.println("own:" + observed + ",foreign:" + outside.PrefixOwner.value);
        } else {
            int observed = origin.LateOwner.observed();
            System.out.println("begin:" + OwnerEvents.calls + ":" + OwnerEvents.before
                    + ",own:" + observed + ",foreign:" + outside.LateOwner.value);
        }
    }
}
