package fixture

// Logical body; README.md defines the native helper and completion model.
object SuspendFinally {
    suspend fun guarded(id: Int): Int {
        return try {
            val first = load(id)
            val second = load(first + 1)
            first + second
        } finally {
            cleanup(id)
        }
    }
    private external suspend fun load(value: Int): Int
    private external fun cleanup(id: Int)
}
