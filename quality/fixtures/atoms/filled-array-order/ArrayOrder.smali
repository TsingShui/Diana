.class public final Lfixture/ArrayOrder;
.super Ljava/lang/Object;

.method private static native next(I)I
.end method

.method private static native make(I)Ljava/lang/Object;
.end method

.method private static native publish([Ljava/lang/Object;)V
.end method

.method private static native caught(ILjava/lang/Object;Ljava/lang/Object;Ljava/lang/Throwable;)V
.end method

.method public static filled()[I
    .locals 3
    const/4 v0, 0x1
    invoke-static {v0}, Lfixture/ArrayOrder;->next(I)I
    move-result v0
    const/4 v1, 0x2
    invoke-static {v1}, Lfixture/ArrayOrder;->next(I)I
    move-result v1
    filled-new-array {v0, v1}, [I
    move-result-object v2
    return-object v2
.end method

.method public static stored()[I
    .locals 3
    const/4 v0, 0x2
    new-array v2, v0, [I
    const/4 v0, 0x1
    invoke-static {v0}, Lfixture/ArrayOrder;->next(I)I
    move-result v0
    const/4 v1, 0x0
    aput v0, v2, v1
    const/4 v0, 0x2
    invoke-static {v0}, Lfixture/ArrayOrder;->next(I)I
    move-result v0
    const/4 v1, 0x1
    aput v0, v2, v1
    return-object v2
.end method

.method public static staged()[Ljava/lang/Object;
    .locals 8
    const/16 v0, 10
    const/4 v1, 0x0
    const/4 v2, 0x0

    :protected_start
    invoke-static {v0}, Lfixture/ArrayOrder;->make(I)Ljava/lang/Object;
    move-result-object v1
    const/16 v0, 20
    move-object v4, v1
    invoke-static {v0}, Lfixture/ArrayOrder;->make(I)Ljava/lang/Object;
    move-result-object v2
    const/16 v0, 30
    move-object v5, v2
    move-object v6, v4
    filled-new-array/range {v4 .. v6}, [Ljava/lang/Object;
    move-result-object v3
    const/16 v0, 40
    invoke-static {v3}, Lfixture/ArrayOrder;->publish([Ljava/lang/Object;)V
    :protected_end
    return-object v3

    :handler
    move-exception v7
    invoke-static {v0, v1, v2, v7}, Lfixture/ArrayOrder;->caught(ILjava/lang/Object;Ljava/lang/Object;Ljava/lang/Throwable;)V
    const/4 v3, 0x0
    return-object v3

    .catchall {:protected_start .. :protected_end} :handler
.end method
