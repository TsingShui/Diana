.class public final Lfixture/LargeArray;
.super Ljava/lang/Object;

.method public static partial(I)[I
    .registers 4
    const v0, 0x7fffffff
    new-array v1, v0, [I
    const/4 v2, 0x0
    aput p0, v1, v2
    return-object v1
.end method

.method public static complete(II)[I
    .registers 5
    const/4 v0, 0x2
    new-array v1, v0, [I
    const/4 v2, 0x0
    aput p0, v1, v2
    const/4 v2, 0x1
    aput p1, v1, v2
    return-object v1
.end method

.method public static dependent(I)[I
    .registers 4
    const/4 v2, 0x2
    new-array v0, v2, [I
    const/4 v1, 0x0
    aput p0, v0, v1
    aget v2, v0, v1
    const/4 v1, 0x1
    aput v2, v0, v1
    return-object v0
.end method
