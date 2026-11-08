package fixture

import kotlin.properties.ReadWriteProperty
import kotlin.reflect.KProperty

/**
 * Atomic fixture: delegated properties, an accessor pair and `lateinit`.
 *
 * Every delegate becomes a `getValue`/`setValue` call through a synthetic field,
 * and the accessors become real methods.
 */
class Case {
    private val cached: String by lazy { "computed" }

    var observed: Int = 0
        set(value) {
            field = value * 2
        }

    var logged: String by LoggingDelegate("init")

    lateinit var ready: String

    fun cache(): String = cached

    fun prepare() {
        ready = "ready"
    }

    fun readyLength(): Int = if (::ready.isInitialized) ready.length else -1
}

class LoggingDelegate(private var stored: String) : ReadWriteProperty<Any?, String> {
    override fun getValue(thisRef: Any?, property: KProperty<*>): String = stored

    override fun setValue(thisRef: Any?, property: KProperty<*>, value: String) {
        stored = value.uppercase()
    }
}
