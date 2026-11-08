package fixture

/**
 * Atomic fixture: Kotlin loop forms.
 *
 * `until`, `downTo`, `step`, `repeat`, `forEach` and labelled `break@`/`continue@`
 * each lower to a different loop shape; the ranges are objects, not integers.
 */
class Case {
    fun until(limit: Int): Int {
        var sum = 0
        for (index in 0 until limit) {
            sum += index
        }
        return sum
    }

    fun downTo(limit: Int): Int {
        var sum = 0
        for (index in limit downTo 1) {
            sum += index
        }
        return sum
    }

    fun stepped(limit: Int): Int {
        var sum = 0
        for (index in 0..limit step 3) {
            sum += index
        }
        return sum
    }

    fun repeated(count: Int): Int {
        var sum = 0
        repeat(count) { index -> sum += index }
        return sum
    }

    fun each(values: List<Int>): Int {
        var sum = 0
        values.forEach { value -> sum += value }
        return sum
    }

    fun labelled(grid: Array<IntArray>): Int {
        var sum = 0
        outer@ for (row in grid) {
            for (value in row) {
                if (value < 0) break@outer
                if (value == 0) continue@outer
                sum += value
            }
        }
        return sum
    }
}
