package fixture;

public final class CheckBranchFloat {
    public static void main(String[] args) {
        double left = Double.parseDouble(args[1]);
        double right = Double.parseDouble(args[2]);
        boolean gate = args.length == 4 && args[3].equals("true");
        switch (args[0]) {
            case "fFalls" -> BranchFloat.fFalls((float) left, (float) right);
            case "fJoinFalls" -> BranchFloat.fJoinFalls((float) left, (float) right);
            case "fJoinBoolean" -> BranchFloat.fJoinBoolean((float) left, (float) right, gate);
            case "dFalls" -> BranchFloat.dFalls(left, right);
            case "dJoinFalls" -> BranchFloat.dJoinFalls(left, right);
            case "iJoinFalls" -> BranchFloat.iJoinFalls(Integer.parseInt(args[1]), Integer.parseInt(args[2]));
            default -> throw new AssertionError(args[0]);
        }
        System.out.println(BranchFloat.trace);
    }
}
