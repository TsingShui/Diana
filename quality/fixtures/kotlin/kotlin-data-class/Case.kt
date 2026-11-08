package fixture

/**
 * Atomic fixture: a data class and the members kotlinc generates for it.
 *
 * `componentN`, `copy`, `copy$default`, `equals`, `hashCode` and `toString` all
 * exist as real methods; a decompiler has to keep them readable next to the
 * declared properties.
 */
data class Case(val name: String, val count: Int) {
    fun describe(): String = "$name:$count"

    fun increase(): Case = copy(count = count + 1)

    fun first(): String {
        val (label, _) = this
        return label
    }
}
