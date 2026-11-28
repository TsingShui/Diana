# Semantic review

The Smali gives the selected arm one producer call. Its result reaches
the common block through a reference copy. The unselected arm forwards
null. Both arms enter the block before the publication call, so the
consumer must use the existing value after publication. The baseline
moved that producer into the later consumer's selected argument and
changed the normal event trace. A publication failure skipped the
producer entirely. A producer failure happened after publication.

The candidate evaluates the selected expression into a local before
publication. The true route records preparation once and carries its
result through publication to the consumer. The false route skips
preparation and passes null. A preparation failure skips both publication
and consumption. A publication failure preserves any earlier preparation
and skips consumption. The handler calls recovery and returns null in
both cases.

The challenge covers the same join inside a cleanup boundary. The
candidate prints separate cleanup calls on normal completion, after
typed recovery, and before rethrow. A failure from consumption escapes
the typed handler and runs cleanup. A failure from recovery runs cleanup
in its nested exceptional path and escapes. Each completed or escaping
model path records one cleanup before its final observed result. Helper
behavior is the authored model. The executed checks cover the listed
faults, argument values, event order, and results under that model.
