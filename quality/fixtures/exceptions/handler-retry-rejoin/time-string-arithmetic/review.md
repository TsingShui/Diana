# Semantic review

The source splits the input on `:` and parses hours only when the resulting
array has exactly three elements. Arrays with at least two elements then take
minutes followed by seconds. The original DEX witnesses retain the split, two
integer conversion sites, and the float conversion. The three recovered
publications preserve the count tests and select the same components for the
modeled paths. [Pinned source](https://github.com/AntennaPod/AntennaPod/blob/175e5ef8fe3e125a5014d70ef0c57cf85661c71b/parser/feed/src/main/java/de/danoeh/antennapod/parser/feed/util/DateUtils.java#L154-L169)

- A lone nonnumeric string and a delimiter-only string reach the zero return
  without number parsing. A trailing empty field drops out of the split, while
  an interior empty minute reaches the integer parser. Four fields select the
  first two without reading the ignored suffixes.
- Hours and minutes use long scaling. Seconds remain a float through scaling
  and convert to long before addition. That produces `16777216000` after
  float rounding of `16777217`, keeps fractional milliseconds for ordinary
  decimal seconds until the conversion, and truncates toward zero.
- With nonzero minutes, NaN contributes zero and leaves the integer part
  intact. Infinities convert to signed long bounds. Adding minutes to the
  positive bound wraps under long arithmetic.
- A failure in both minutes and seconds comes from the integer parse. Valid
  minutes allow the float parse to fail. The hours error is also reached
  before the seconds parser on the three-component route. A null argument
  faults at the initial split.

The comparison control returned `123000` for `02:03.5` and reported the float
parser for two invalid components. Its early cast also changed the signed
infinity outcomes and the addition after conversion. These observations
exercise separate arithmetic and fault-order obligations in the current model.
