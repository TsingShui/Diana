# Quiet NaN payload review

The original float and double return bodies load raw bits and return them
directly. The joined float selection retains two distinct quiet NaNs and
passes the original selected bits to `Float.floatToRawIntBits`. The static
store writes the original negative double NaN at method execution time.

The protected double selection similarly carries two distinct quiet NaNs.
Both branches join at `inspectDouble`. The helper writes the original bits
before checking the exception argument. Without a fault, the caller returns
those bits and the caught field stays null. With a fault, the typed catch
stores the same argument and returns the bits the helper already wrote.

Diana 0.4.21 renders the noncanonical payloads as canonical positive NaN
constants. The two float selection paths agree numerically as NaNs but
lose distinct observable bits. The return, store and protected-helper
paths receive canonical bits. The caught exception identity survives.
The canonical compatibility control agrees.

The repaired publication uses `Float.intBitsToFloat` and
`Double.longBitsToDouble` with the original literal words. The original
canonical float control still publishes `Float.NaN`. The repaired Java
agrees with all ten scenarios and reproduces the protected write, return
bits and caught exception identity.

The raw DEX model checks the fixture's exact instruction and exception
bindings. The JVM observer executes the authored and recovered classes
in scratch storage. These checks cover the selected quiet NaNs.
