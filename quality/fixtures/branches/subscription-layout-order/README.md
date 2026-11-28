# Subscription layout construction order model

This model selects AntennaPod 3.12.3 method `setColumnNumber(I)V` on
`Lde/danoeh/antennapod/ui/screen/subscriptions/SubscriptionFragment;`,
physical locator `e6:l0:c1031:m6631` in `classes3.dex`. The pinned APK has
SHA-256 `6e79f28f69cef1a8b2e4f5f88c9c3cf3c3d7c51356f01394ac23b0dc5d56a00b`.
The source file has SHA-256
`0d35a96da963bd24f39653fe879527290c302e5cc05638145407a2cc7cac643f`.
[Original method at lines 302–321](https://github.com/AntennaPod/AntennaPod/blob/175e5ef8fe3e125a5014d70ef0c57cf85661c71b/app/src/main/java/de/danoeh/antennapod/ui/screen/subscriptions/SubscriptionFragment.java#L302-L321)

The original DEX body contains 96 code units. A prior decoration is removed
at code unit 6 and the field is cleared at 10. The single-column path reads
the default at 16. The tablet path constructs its two-span layout at 30
and jumps to the shared tail. The handset path reads the context at 38,
constructs its one-span layout at 42 and jumps to the same tail at 67.
The multi-column path constructs a layout at 52 and adds the new decoration
at 64. Every normal path reaches the adapter setter at 69 before installing
the layout at 74. The method contains no exception handlers.

The model uses the exact original or recovered method with independently
authored recording stubs. Each scenario starts with adapter, layout and
preference values of seven. An optional prior decoration starts attached
and referenced by the fragment field. The stubs record constructor arguments,
decoration identity, adapter and layout updates, preference writes, and
toolbar refresh. A selected layout constructor can throw before returning.
The harness catches that fault and records the fragment's final modeled
state. [Scenario inputs and expected traces](model.json)

The pinned source and JADX 1.5.7 both passed all nine scenarios. Diana 0.4.2
passed six. On each normal handset scenario Diana updates the adapter before
reading the context and constructing the layout. The tablet and multi-column
controls preserve their order. In the throwing handset scenario, source and
JADX leave the prior adapter value at seven. Diana leaves it at one. All
three remove the old decoration and retain the prior layout and preference
values. The injected failure tests the effect of a throwing constructor
under the stubs.

```sh
python3 quality/decbench run --corpus antennapod-3.12.3 \
  --binary target/debug/diana --execute-models --at e6:l0:c1031:m6631
```
