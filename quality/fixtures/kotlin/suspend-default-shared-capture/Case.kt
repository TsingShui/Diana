package fixture

// Logical body. The extra invalid-state effect is specified in README.md.
object SuspendCapturedDefault {
    suspend fun choose(seed: Int): Int {
        val first = load(seed)
        record(first)
        if (first < 0) return -1
        val second = load(first + 1)
        record(second)
        return first + second + seed
    }
    private external suspend fun load(value: Int): Int
    private external fun record(value: Int)
}
