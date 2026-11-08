package fixture

/**
 * Atomic fixture: Kotlin null-safety forms and the DEX they produce.
 *
 * `?.` becomes a null check plus a call, `?:` becomes a branch with a default,
 * `!!` becomes `Intrinsics.checkNotNull` on the receiver, and a chain of safe
 * calls becomes label branches. `@kotlin.Metadata` is what identifies the class as
 * Kotlin; the fixture keeps it so the output can be judged against it.
 */
class Case {
    fun lengthOrZero(text: String?): Int = text?.length ?: 0

    fun requireLength(text: String?): Int = text!!.length

    fun plainParameter(count: Int): Int = count + 1

    fun chained(text: String?): String? = text?.trim()?.lowercase()
}
