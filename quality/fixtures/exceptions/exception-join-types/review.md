# Semantic review

For the zero-step path the input has a null saved exception, so it
returns without making a write. The recovered null initialization, loop
condition and post-loop check preserve that route. A successful write
leaves the saved exception unchanged. The next iteration can therefore
retain a prior exception through an intervening success.

On an IOException, the distinct input handler assigns the caught object
and calls the IO observer. A RuntimeException uses the runtime observer.
The candidate uses the original types and keeps the recorded object for
the next loop header. The runtime assignment can print after its native
observer. If the observer completes as modeled, the assignment still
reaches the next iteration. If it throws, the method exits and no following
path observes the local assignment. The shared landing pad recovers one
catch with both original alternatives and invokes the single common observer.

After the loop the source and candidate pick IOException by an instance
test and use the RuntimeException path otherwise. Both throw operands use
the last saved reference. Unknown native behavior is outside the observed
execution evidence. The Engine check verifies the recovered representation
and no diagnostic. It is a structural check.
