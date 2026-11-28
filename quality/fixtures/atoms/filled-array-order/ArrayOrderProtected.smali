.class public final Lfixture/ArrayOrderProtected;
.super Ljava/lang/Object;

.method private static native make(I)Ljava/lang/Object;
.end method

.method private static native caught(ILjava/lang/RuntimeException;)V
.end method

.method public static protectedFilled()[Ljava/lang/Object;
    .locals 4
    const/16 v3, 80
    :protected_start
    const/16 v0, 10
    invoke-static {v0}, Lfixture/ArrayOrderProtected;->make(I)Ljava/lang/Object;
    move-result-object v0
    move-object v2, v0
    const/16 v1, 20
    invoke-static {v1}, Lfixture/ArrayOrderProtected;->make(I)Ljava/lang/Object;
    move-result-object v1
    filled-new-array/range {v0 .. v2}, [Ljava/lang/Object;
    move-result-object v0
    :protected_end
    return-object v0

    :handler
    move-exception v0
    invoke-static {v3, v0}, Lfixture/ArrayOrderProtected;->caught(ILjava/lang/RuntimeException;)V
    const/4 v0, 0x0
    return-object v0

    .catch Ljava/lang/RuntimeException; {:protected_start .. :protected_end} :handler
.end method
