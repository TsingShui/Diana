package fixture;

public final class CheckGuardedSourceOrder {
    public static void main(String[] args) {
        String method = args[0];
        for (int index = 1; index < args.length; index += 2) {
            String facts = args[index];
            int mode = Integer.parseInt(args[index + 1]);
            Object input = facts.equals("plain") ? new Object() : new GuardedSourceOrder.Selection(
                facts.equals("first") || facts.equals("both"),
                facts.equals("second") || facts.equals("both"));
            GuardedSourceOrder.Effects.log = "";
            Object value = method.equals("collect")
                ? GuardedSourceOrder.collect(input, mode)
                : GuardedSourceOrder.collectAfterOpening(input, mode);
            String remote = input instanceof GuardedSourceOrder.Selection
                ? Boolean.toString(((GuardedSourceOrder.Selection) input).remote)
                : "none";
            System.out.println("trace:" + GuardedSourceOrder.Effects.log + ",result:" + value + ",remote:" + remote);
        }
    }
}
