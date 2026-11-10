package fixture;

// Logical source explanation; the Smali contains the original ABI protocol.
final class AssociatedBody {
    int seed;

    Object logicalBody() {
        int savedSeed = seed;
        int prepared = AssociationEffects.prepare(savedSeed);
        Number first = (Number) suspendLoad(prepared);
        int firstReply = first.intValue();
        AssociationEffects.record(firstReply, savedSeed);
        if (firstReply < 0) {
            return Integer.valueOf(-1);
        }
        Number second = (Number) suspendLoad(firstReply);
        AssociationEffects.observe(second.intValue(), savedSeed, firstReply);
        return second;
    }

    // A logical suspension completion, not an implementation of the ABI call.
    native Object suspendLoad(int key);
}

final class AssociationEffects {
    static native int prepare(int seed);
    static native Object load(int key, Object continuation);
    static native void record(int reply, int seed);
    static native void observe(int reply, int seed, int firstReply);
}

// BadResume.smali adds record(savedSeed, savedSeed) only on the first resume
// ingress, after throwOnFailure. Its immediate and resumed paths have different
// effects, so that member must retain the explicit ABI protocol.

// PublicationBody.smali explains this captureless logical sequence:
// Number first = (Number) suspendLoad(5);
// return suspendLoad(first.intValue());
// Both result edges have one publication, while both marker edges share one
// original return. The shared marker return must have one static owner.
// SwitchPublicationBody.smali expresses the same member with sparse dispatch.
