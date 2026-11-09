package fixture

object SuspendThree {
    suspend fun walk(seed: Int): Int {
        val first = load(seed)
        record(first)
        if (first < 0) return -1
        val second = load(first + 1)
        record(second)
        if (second == 0) return first
        val third = load(first + second)
        record(third)
        return first + second + third
    }
    private external suspend fun load(value: Int): Int
    private external fun record(value: Int)
}
