# Wide archive counts and entries

The fixture uses Python 3's ZIP writer and the independently authored class
assembler from [physical-inventory](../physical-inventory/README.md). It
generates disposable files in `.cache/fixtures/large-archives/`.

| Input | Established facts |
|---|---|
| `large.jar` | The archive has 70,002 records. A false end signature lies inside the final comment. The only classes are `fixture/archive/Large` at `j70000` and `fixture/archive/Neighbor` at `j70001`. |
| `wide.jar` | `Large` uses four ZIP64 replacements in its central record and two at its local header. An unknown extra field precedes the replacements. The ZIP64 end record includes a length-delimited extension and agrees with the small legacy end fields. |
| `bad-local.jar`, `bad-extra.jar`, `bad-legacy.jar` | A local size contradicts the central record or its local extra field overruns its declared extent. Only `j0` is unavailable. |
| `huge-count.jar`, `bad-locator.jar` | The declared count or locator exceeds the checked directory bounds. No central records are established. |
| `stream.jar` | Its ZIP64 local sizes are zero. The data-descriptor bit is set and the central directory supplies the sizes of the valid deflate stream. |

The first method in `Large` returns 41. The only method in `Neighbor`
returns 22. Each method consists of `bipush` and `ireturn`.

```sh
python3 quality/fixtures/jvm/large-archives/prepare.py fields .cache/fixtures/large-archives
python3 quality/fixtures/jvm/large-archives/prepare.py count .cache/fixtures/large-archives
python3 quality/fixtures/jvm/large-archives/prepare.py stream .cache/fixtures/large-archives
cargo nextest run --locked -p diana --test jvm_large_archives_test
```

The recipe independently opens the valid archives with Python's standard
library. For the count case it tests the original records without the deliberately
embedded end signature in the comment. The CLI regression tests retain the
original comment and recover original coordinates, source, and local
unavailability.
