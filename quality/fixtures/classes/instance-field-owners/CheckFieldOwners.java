package fixture;

public final class CheckFieldOwners {
    private static ShadowFields seed(int root, int parent, int own) {
        ShadowFields receiver = new ShadowFields();
        ((RootFields) receiver).value = root;
        ((ParentFields) receiver).value = parent;
        receiver.value = own;
        return receiver;
    }

    private static String snapshot(String name, ShadowFields receiver) {
        return name + ":" + ((RootFields) receiver).value + ":"
                + ((ParentFields) receiver).value + ":" + receiver.value;
    }

    public static void main(String[] args) {
        String scenario = args[0];
        ShadowFields receiver = seed(10, 20, 30);
        ShadowFields peer = seed(40, 50, 60);
        String result;
        try {
            result = switch (scenario) {
                case "own-read" -> "return:" + receiver.readOwn();
                case "parent-read" -> "return:" + receiver.readParent();
                case "root-read" -> "return:" + receiver.readRoot();
                case "parent-write" -> {
                    receiver.writeParent(77);
                    yield "return:void";
                }
                case "root-write" -> {
                    receiver.writeRoot(77);
                    yield "return:void";
                }
                case "peer-distinct" -> "return:" + receiver.writePeerRoot(peer, 77);
                case "peer-alias" -> "return:" + receiver.writePeerRoot(receiver, 77);
                case "peer-null" -> "return:" + receiver.writePeerRoot(null, 77);
                case "external-read" -> "return:" + ForeignFieldAccess.readRootThroughShadow(receiver);
                case "external-null" -> "return:" + ForeignFieldAccess.readRootThroughShadow(null);

                case "chosen-peer" -> {
                    ForeignFieldAccess.writeChosenRoot(peer, 77);
                    yield "return:void";
                }
                case "chosen-null" -> {
                    ForeignFieldAccess.writeChosenRoot(null, 77);
                    yield "return:void";
                }
                default -> throw new AssertionError(scenario);
            };
        } catch (NullPointerException fault) {
            result = "throw:NullPointerException";
        }
        String trace = result + "," + snapshot("self", receiver) + "," + snapshot("peer", peer);
        if (scenario.startsWith("chosen-")) {
            trace += ",calls:" + ForeignFieldAccess.calls;
        }
        System.out.println(trace);
    }
}
