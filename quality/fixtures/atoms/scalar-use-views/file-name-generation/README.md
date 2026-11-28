# File-name generation model

This opt-in DecBench model covers `FileNameGenerator.generateFileName(String)`
in AntennaPod 3.12.3. The F-Droid free APK identifies upstream revision
`175e5ef8fe3e125a5014d70ef0c57cf85661c71b`. Its original selector is
`e4:l0:c8167:m54687`, and its full member is
`Lde/danoeh/antennapod/net/download/serviceinterface/FileNameGenerator;->generateFileName(Ljava/lang/String;)Ljava/lang/String;`.
The original declaration, algorithm and license are available in the pinned
[upstream source](https://github.com/AntennaPod/AntennaPod/blob/175e5ef8fe3e125a5014d70ef0c57cf85661c71b/net/download/service-interface/src/main/java/de/danoeh/antennapod/net/download/serviceinterface/FileNameGenerator.java#L32-L53)
and [license](https://github.com/AntennaPod/AntennaPod/blob/175e5ef8fe3e125a5014d70ef0c57cf85661c71b/LICENSE).

An independent DEX read resolves the character test at code unit 52 to
`ArrayUtils.contains([CC)Z` and the following append at code unit 58 to
`StringBuilder.append(C)Ljava/lang/StringBuilder;`. The empty branch calls
`randomString(8)` at 80. The long branch calls `substring` with end index 209
at 100 and calls `md5(String)` at 112. The
[AntennaPod review](../../../../reports/antennapod-3.12.3.md) records the
branch and argument evidence.

## Expected observations

The independently authored [model](model.json) records the exact return value
and ordered random and digest calls. Both the pinned source control and the
selected recovered method run in the same [Java template](FileNameGenerator.java).

| Scenario | Expected result and effects |
| --- | --- |
| Single ASCII character | Input `A` returns `A`. |
| ASCII allowlist and space runs | Retained letters, digits, underscores and hyphens return as `A _-- Z9`. Leading, duplicate and trailing spaces are removed. |
| Decomposable accents and compatibility forms | The mixed Unicode input returns `Angstrom cafe C`. Accents are stripped, the full-width C becomes ASCII, and the symbol and emoji are filtered. |
| Unicode spaces | Non-breaking and thin spaces normalize to ordinary spaces. The line separator remains outside the allowlist. |
| Empty or fully rejected input | Each case calls `randomString(8)` once and returns the supplied eight-character value. |
| Length 241 | The entire accepted name is returned and neither helper runs. |
| Length 242 | The digest helper receives all 242 accepted characters. The result is the first 209 characters, an underscore and `25c72835b3f9f11c9183be3b09dd6aa1`. |
| Raw length 242, accepted length 241 | A rejected trailing period leaves 241 characters. The digest helper is skipped. |

The template builds the allowlist from ASCII ranges and uses the Java
`char` overload for the membership model. Accent stripping uses the
[Commons Lang 3.18.0 normalization rule](https://github.com/apache/commons-lang/blob/rel/commons-lang-3.18.0/src/main/java/org/apache/commons/lang3/StringUtils.java#L7919-L7925)
for these inputs. The MD5 helper uses the JDK provider on UTF-8 bytes and
records its argument. The random helper supplies the fixed value `B7_r-N0q`
and records the requested length. The scenarios exercise the selected method's
fallback selection and arguments. The application's random number generation,
other Unicode mappings and Android execution remain outside the model's scope.

## Executed evidence

The APK SHA-256 was
`6e79f28f69cef1a8b2e4f5f88c9c3cf3c3d7c51356f01394ac23b0dc5d56a00b`.
OpenJDK 21.0.12.1 executed the model with the independently checked source
span at lines 32–53. The source control matched all ten authored expectations.

| Selected method publication | Modeled result |
| --- | --- |
| Diana 0.4.2 release, SHA-256 `970abe217e5828594d737605e28c96bdbb4d0b65531f55789da79c010f08ff49` | Ten pass. |
| JADX 1.5.7, SHA-256 `31d8d8add65e5c7fcc130c9acc006a45466a1345155d722fb6577e0975136ecf` | Ten pass. |
| Diana 0.1.0 at `b214ec60`, SHA-256 `1cbbedb53f03491280c959a52ca22584ae32fc58a8bb9d910a9c0e4706abaac0` | Ten unchecked. The Java compiler rejects the explicit `goto` statements in the original publication. |
| Source control with only the character append argument widened to `int` | Eight fail and two pass. Input `A` returns `65`. The two fallback cases still invoke `randomString(8)` and return the expected token. |

The two current publications preserve the source loop paths. Each scan checks
the last retained character before appending a permitted space and advances
after both accepted and rejected characters. Both use a character argument
for the append and retain the ordered random and digest calls at the same
boundaries. The historical publication shows an integer append argument, but
its `goto` notation precludes the modeled execution. The widened source
variant separately verifies sensitivity to that overload change.

After preparing the public corpus, select this method through the
[DecBench CLI](../../../../decbench/README.md) with `--execute-models` and
`--at e4:l0:c8167:m54687`. The historical CLI accepts the full member via
`get-method`, and its source can be inspected with `--format text`. Keep the
APK, upstream source archive and generated reports in the ignored cache.
