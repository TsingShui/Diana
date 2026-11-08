package fixture

/**
 * Atomic fixture: suspend functions and the state machine kotlinc emits.
 *
 * A `suspend` call is compiled through `Continuation` with a label switch, so the
 * body becomes `invokeSuspend` with a `when(label)` and a `COROUTINE_SUSPENDED`
 * check.
 */
class Case {
    suspend fun fetch(id: Int): Int {
        val first = load(id)
        if (first < 0) {
            return -1
        }
        val second = load(first + 1)
        return second + first
    }

    suspend fun total(limit: Int): Int {
        var sum = 0
        var index = 0
        while (index < limit) {
            sum += load(index)
            index++
        }
        return sum
    }

    suspend fun guarded(id: Int): Int {
        return try {
            load(id)
        } catch (error: IllegalStateException) {
            -1
        }
    }

    private suspend fun load(value: Int): Int {
        if (value < 0) {
            throw IllegalStateException("negative")
        }
        return value * 2
    }
}
