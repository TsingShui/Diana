.class public final Lfixture/FloatCompare;
.super Ljava/lang/Object;
.field public static trace:I

.method public static left()F
    .locals 1
    sget v0, Lfixture/FloatCompare;->trace:I
    mul-int/lit8 v0, v0, 10
    add-int/lit8 v0, v0, 1
    sput v0, Lfixture/FloatCompare;->trace:I
    const/high16 v0, 0x40000000
    return v0
.end method

.method public static right()F
    .locals 1
    sget v0, Lfixture/FloatCompare;->trace:I
    mul-int/lit8 v0, v0, 10
    add-int/lit8 v0, v0, 2
    sput v0, Lfixture/FloatCompare;->trace:I
    const/high16 v0, 0x3f800000
    return v0
.end method

.method public static one()Z
    .locals 4
    invoke-static {}, Lfixture/FloatCompare;->left()F
    move-result v0
    invoke-static {}, Lfixture/FloatCompare;->right()F
    move-result v1
    cmpg-float v2, v0, v1
    const/4 v3, 1
    if-ne v2, v3, :no
    return v3
    :no
    const/4 v3, 0
    return v3
.end method

.method public static zero()Z
    .locals 3
    invoke-static {}, Lfixture/FloatCompare;->left()F
    move-result v0
    invoke-static {}, Lfixture/FloatCompare;->right()F
    move-result v1
    cmpg-float v2, v0, v1
    if-lez v2, :no
    const/4 v2, 1
    return v2
    :no
    const/4 v2, 0
    return v2
.end method

.method public static value()I
    .locals 3
    invoke-static {}, Lfixture/FloatCompare;->left()F
    move-result v0
    invoke-static {}, Lfixture/FloatCompare;->right()F
    move-result v1
    cmpg-float v2, v0, v1
    return v2
.end method

.method public static leftNaN()F
    .locals 1
    sget v0, Lfixture/FloatCompare;->trace:I
    mul-int/lit8 v0, v0, 10
    add-int/lit8 v0, v0, 1
    sput v0, Lfixture/FloatCompare;->trace:I
    const/high16 v0, 0x7fc00000
    return v0
.end method

.method public static rightMaybe(Ljava/lang/RuntimeException;)F
    .locals 1
    sget v0, Lfixture/FloatCompare;->trace:I
    mul-int/lit8 v0, v0, 10
    add-int/lit8 v0, v0, 2
    sput v0, Lfixture/FloatCompare;->trace:I
    if-eqz p0, :ok
    throw p0
    :ok
    const/high16 v0, 0x3f800000
    return v0
.end method

.method public static nanSkips(Ljava/lang/RuntimeException;)Z
    .locals 4
    invoke-static {}, Lfixture/FloatCompare;->leftNaN()F
    move-result v0
    invoke-static {p0}, Lfixture/FloatCompare;->rightMaybe(Ljava/lang/RuntimeException;)F
    move-result v1
    cmpg-float v2, v0, v1
    const/4 v3, 1
    if-ne v2, v3, :no
    return v3
    :no
    const/4 v3, 0
    return v3
.end method

.method public static lowFloatZero(Ljava/lang/RuntimeException;)Z
    .locals 3
    invoke-static {}, Lfixture/FloatCompare;->leftNaN()F
    move-result v0
    invoke-static {p0}, Lfixture/FloatCompare;->rightMaybe(Ljava/lang/RuntimeException;)F
    move-result v1
    cmpl-float v2, v0, v1
    if-gez v2, :no
    const/4 v2, 1
    return v2
    :no
    const/4 v2, 0
    return v2
.end method

.method public static doubleLeft(Z)D
    .locals 2
    sget v0, Lfixture/FloatCompare;->trace:I
    mul-int/lit8 v0, v0, 10
    add-int/lit8 v0, v0, 1
    sput v0, Lfixture/FloatCompare;->trace:I
    if-eqz p0, :finite
    const-wide/high16 v0, 0x7ff8000000000000L
    return-wide v0
    :finite
    const-wide/high16 v0, 0x4000000000000000L
    return-wide v0
.end method

.method public static doubleRightMaybe(Ljava/lang/RuntimeException;)D
    .locals 2
    sget v0, Lfixture/FloatCompare;->trace:I
    mul-int/lit8 v0, v0, 10
    add-int/lit8 v0, v0, 2
    sput v0, Lfixture/FloatCompare;->trace:I
    if-eqz p0, :ok
    throw p0
    :ok
    const-wide/high16 v0, 0x3ff0000000000000L
    return-wide v0
.end method

.method public static doubleFinite()Z
    .locals 6
    const/4 v5, 0
    invoke-static {v5}, Lfixture/FloatCompare;->doubleLeft(Z)D
    move-result-wide v0
    invoke-static {v5}, Lfixture/FloatCompare;->doubleRightMaybe(Ljava/lang/RuntimeException;)D
    move-result-wide v2
    cmpg-double v4, v0, v2
    const/4 v5, 1
    if-ne v4, v5, :no
    return v5
    :no
    const/4 v5, 0
    return v5
.end method

.method public static doubleHigh(Ljava/lang/RuntimeException;)I
    .locals 6
    const/4 v5, 1
    invoke-static {v5}, Lfixture/FloatCompare;->doubleLeft(Z)D
    move-result-wide v0
    invoke-static {p0}, Lfixture/FloatCompare;->doubleRightMaybe(Ljava/lang/RuntimeException;)D
    move-result-wide v2
    cmpg-double v4, v0, v2
    return v4
.end method

.method public static doubleLow(Ljava/lang/RuntimeException;)Z
    .locals 6
    const/4 v5, 1
    invoke-static {v5}, Lfixture/FloatCompare;->doubleLeft(Z)D
    move-result-wide v0
    invoke-static {p0}, Lfixture/FloatCompare;->doubleRightMaybe(Ljava/lang/RuntimeException;)D
    move-result-wide v2
    cmpl-double v4, v0, v2
    const/4 v5, -1
    if-ne v4, v5, :no
    const/4 v5, 1
    return v5
    :no
    const/4 v5, 0
    return v5
.end method
