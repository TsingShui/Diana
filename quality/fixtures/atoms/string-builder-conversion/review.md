# Semantic review

The original DEX obtains both objects before creating the builder or
buffer. Object conversion is visible when each later append calls the
object's `toString`. A producer failure therefore skips both conversions.
When neither producer fails, the first object's callback precedes the
second object's callback and may prevent it from being called.

The 0.4.11 recovered expressions evaluate each producer as an operand
of a left-associative string concatenation. The first object's callback
therefore precedes the second producer. The success cases retain the
final text and report a different effect order. The second-producer
failure now occurs after an extra first conversion. A first conversion
failure skips the second producer, and wins over a simultaneously
configured producer failure. The protected variant chooses its
conversion handler in this situation where the original chooses its
producer handler.

The first-producer failure and the null first object retain the original
observations. The exact unmatched exception still escapes the protected
method, although it follows an extra conversion in the recovered path.

The 0.4.12 output computes both objects in local initializers and retains
the two `append(Object)` calls. Each producer fault now exits before any
conversion. When both producers succeed, the first append converts its
argument before the second append can run. Each typed handler receives
the original fault object, while an unmatched fault still escapes. A null
argument uses the builder's normal null rendering and skips the callback.

The executed checks compare decoded invokes and typed handler targets
using modeled library and producer hooks. The Java reference and
recovered methods use the actual JDK libraries. Resource exhaustion
during allocation is outside these scenarios.
