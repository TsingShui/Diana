# Semantic review

The original FairEmail 1.2338 method `HtmlHelper.autoLink(Document, boolean)`
calls `Pattern.compile` at `0x40` and allocates the visitor at `0x44`. The
initializer follows at `0x46`. The baseline nests `Pattern.compile` inside
the visitor arguments. The candidate stores the pattern before `new` and
passes it to the same visitor before calling `NodeTraversor.traverse`. This
is an instruction and output comparison for the public APK.

The challenge makes the allocation observable through `PrefixValue.<clinit>`.
The candidate evaluates the nested preparation calls from the inside out and
computes the first argument into a local. Doing the pure addition before
allocation preserves the argument value. The doubling remains in the second
argument, and the same initializer and field read follow. The six scenarios
execute the exact recovered method body with the authored Java helpers.

- On cold success the two preparations complete before class initialization.
  The constructor records both derived integers and the field read returns
  their encoded result. A warm success repeats both preparation calls and
  construction without repeating class initialization.
- On a preparation fault the second call has already logged its arguments.
  The exception escapes before allocating or initializing the class. A retry
  initializes the class if the failure left it cold, and uses the existing
  initialized class if the failure occurred after a successful call.
- On a class initializer fault both preparations complete before the recorded
  initializer failure. The retry repeats both preparation calls and then
  fails when it uses the erroneous class. The constructor is skipped in
  both calls.
- On a constructor fault the preparation calls and class initialization
  complete. The initializer logs the original argument values and throws
  before the field read. The retry can construct and read the field using
  the already initialized class.

The earlier `constructor-argument-effects` fixture also exercises the
opposite dependency. Its `tree` and `caughtError` methods compute observable
values after allocation, with unchanged recovery text on 0.4.7 and 0.4.8.
The new movement boundary applies when the CFG proves a straight-line path
with no observable operation between allocation and initialization.
Per-site catch snapshots for split construction remain an open case.
