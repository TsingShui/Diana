public final class CheckFieldOwners {
    private static String slots(FieldChild object) {
        return ((FieldParent) object).slot + "," + object.slot;
    }

    private static String nullAccess(boolean write) {
        FieldPeer.evaluations = 0;
        try {
            if (write) {
                FieldPeer.writePeer(null, 44);
            } else {
                FieldPeer.readPeer(null);
            }
            return "returned," + FieldPeer.evaluations;
        } catch (NullPointerException expected) {
            return "NullPointerException," + FieldPeer.evaluations;
        }
    }

    private static void observe(String name, String value) {
        System.out.println(name + "\t" + value);
    }

    public static void main(String[] args) {
        FieldChild child = new FieldChild();
        observe("dispatch", child.callParent() + "," + child.dispatch());
        observe("parent-before", child.readParent() + "," + slots(child));
        child.writeParent(49);
        observe("parent-write", child.readParent() + "," + slots(child));
        observe("both-slots", child.writeThenReadBoth(11) + "," + slots(child));
        FieldChild peer = new FieldChild();
        FieldPeer.evaluations = 0;
        observe("peer-read", FieldPeer.readPeer(peer) + "," + FieldPeer.evaluations);
        FieldPeer.writePeer(peer, 66);
        observe("peer-write", slots(peer) + "," + slots(child) + "," + FieldPeer.evaluations);
        observe("peer-null-read", nullAccess(false));
        observe("peer-null-write", nullAccess(true));
    }
}
