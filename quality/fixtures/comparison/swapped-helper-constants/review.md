# Semantic review

The ordinary get-class outputs for x.a and y.s each return their own helper applied to Integer.reverseBytes of the input. Reading those helpers establishes XOR 7 and XOR 15. The x.b and y.t outputs each call Integer.lowestOneBit before their respective XOR helper, whose constants change in the opposite direction. Both routes use the returned library value once. There is no other call, branch or handler on these paths.

In each router the external call and argument use supply evidence independent of the helper body. Reading the two arithmetic instructions in isolation would invert the owner pairing and falsely attribute the observable change to the external call site. This review compared actual decompiled output against the Java/Smali input. It did not execute the fixture on Android.

The revised comparison matched reverseBytes to reverseBytes and lowestOneBit to lowestOneBit in their respective routers. Both routers were unchanged after mapping their helper references. Each changed helper reported its own XOR at offset 0, with 7 changing to 15 in the first route and 15 changing to 7 in the other. The reported sites therefore account for the changed returns on both reviewed paths.
