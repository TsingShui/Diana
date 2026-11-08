package fixture

/**
 * Atomic fixture: sealed hierarchy with exhaustive `when`.
 *
 * The compiler lowers `when` to a type/tag dispatch plus smart casts; the sealed
 * subclasses are real classes.
 */
sealed class Case {
    data class Ready(val value: Int) : Case()

    data class Failed(val reason: String) : Case()

    object Pending : Case()

    fun describe(): Int =
        when (this) {
            is Ready -> value
            is Failed -> reason.length
            Pending -> 0
        }

    fun labelled(): String {
        val current = this
        return when (current) {
            is Ready -> "ready:${current.value}"
            is Failed -> "failed:${current.reason}"
            Pending -> "pending"
        }
    }
}
