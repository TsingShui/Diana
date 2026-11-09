package fixture

// Logical body; README.md defines the native helper and completion model.
object SuspendBranch {
    suspend fun choose(seed: Int): Int {
        val first = load(seed)
        record(first)
        if (first < 0) return -1
        val second = load(first + 1)
        record(second)
        return first + second
    }
    private external suspend fun load(value: Int): Int
    private external fun record(value: Int)
}
