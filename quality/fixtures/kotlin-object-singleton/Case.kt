package fixture

/**
 * Atomic fixture: `object` singleton and `companion object`.
 *
 * Both compile to a class with a static `INSTANCE` field and a private
 * constructor; the companion also gets a nested holder class.
 */
object Counter {
    var total: Int = 0
        private set

    fun add(delta: Int): Int {
        total += delta
        return total
    }
}

class Case(val name: String) {
    companion object {
        const val PREFIX: String = "case:"

        var created: Int = 0
            private set

        fun make(name: String): Case {
            created++
            return Case(PREFIX + name)
        }
    }

    fun label(): String = PREFIX + name
}
