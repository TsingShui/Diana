package fixture

// Logical body; README.md defines the native helper and completion model.
object SuspendTwoSites {
    suspend fun total(limit: Int): Int {
        var sum = 0
        var index = 0
        while (index < limit) {
            val first = load(index)
            record(first)
            if (first < 0) break
            val second = load(first + 1)
            sum += first + second
            index++
        }
        return sum
    }
    private external suspend fun load(value: Int): Int
    private external fun record(value: Int)
}
