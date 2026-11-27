# Multidex comparison publication

`prepare.py OUTPUT.apk` packs the prepared DEX files from six atomic fixtures
using Python's standard ZIP writer. The roots contain 9, 9, 8, 10, 8 and 8
defined methods in archive order. Their Java sources and original DEX files
remain with the atomic fixtures.

Comparing the archive with a copy exposes all 52 old and all 52 new method
locations. Both presentations preserve the side, signature and locator of every
method, including the final root. The command accepts a distinct output file
and refuses an output file that aliases either input.
