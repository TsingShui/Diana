.class public final Lfixture/DexConsumer;
.super Ljava/lang/Object;

.method public static compileText()Ljava/lang/String;
    .locals 1
    const-string v0, "compiled"
    return-object v0
.end method

.method public static compileNumber()I
    .locals 1
    const/16 v0, 42
    return v0
.end method

.method public static runtimeText()Ljava/lang/String;
    .locals 1
    sget-object v0, Lfixture/DexProvider;->RUNTIME_TEXT:Ljava/lang/String;
    return-object v0
.end method

.method public static runtimeNumber()I
    .locals 1
    sget v0, Lfixture/DexProvider;->RUNTIME_NUMBER:I
    return v0
.end method
