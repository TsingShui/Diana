.class public final Lfixture/ForeignFieldAccess;
.super Ljava/lang/Object;
.field public static calls:I

.method public static readRootThroughShadow(Lfixture/ShadowFields;)I
    .locals 1
    iget v0, p0, Lfixture/RootFields;->value:I
    return v0
.end method

.method public static pick(Lfixture/ShadowFields;)Lfixture/ShadowFields;
    .locals 1
    sget v0, Lfixture/ForeignFieldAccess;->calls:I
    add-int/lit8 v0, v0, 1
    sput v0, Lfixture/ForeignFieldAccess;->calls:I
    return-object p0
.end method

.method public static writeChosenRoot(Lfixture/ShadowFields;I)V
    .locals 1
    invoke-static {p0}, Lfixture/ForeignFieldAccess;->pick(Lfixture/ShadowFields;)Lfixture/ShadowFields;
    move-result-object v0
    iput p1, v0, Lfixture/RootFields;->value:I
    return-void
.end method
