# Semantic review

`PublicationFace.<clinit>()V` initializes one object and stores it in
`FIRST` before calling `PublicationFace.observe()V`. The same-thread call
reads the already-published interface field and records the object in
`HolderObservation.seen`. The original local reaches the `SECOND` store
after the call. The original fields and recorded reference therefore alias.

Diana 0.4.25 places both writes in a synthetic holder. The callback reads
the actual interface field, which is still null while the initializer
waits for the holder to return. The authored and exact baseline Java runs
confirmed the difference even though `FIRST` and `SECOND` alias after both
initializers finish.

Diana 0.4.26 debug source retains the single local, the original interface
initializer and the field-write order around the callback. Its observer
still reads `PublicationFace.FIRST`. This path comparison establishes the
available Java-like schedule. The candidate source was not recompiled or
executed. The `HolderTail` publication is byte-identical across the two
Diana versions and still assigns one allocated object to both aliases.
