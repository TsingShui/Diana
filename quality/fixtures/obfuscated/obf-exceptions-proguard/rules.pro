# obfuscator: proguard
-dontwarn
-dontshrink
-dontoptimize
-printmapping mapping.txt
-keep,allowobfuscation class fixture.ComplexExceptions { *; }
-keep,allowobfuscation class fixture.ComplexExceptions$Res { *; }
