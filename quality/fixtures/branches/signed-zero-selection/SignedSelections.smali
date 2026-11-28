.class public final Lfixture/SignedSelections;
.super Ljava/lang/Object;

.field public static staticFloat:F
.field public static staticDouble:D
.field public static lastBits:I
.field public static caught:Ljava/lang/ArithmeticException;

.method public static rawFloat(Z)I
    .locals 1
    if-eqz p0, :positive
    const/high16 v0, -0x80000000
    goto :join
    :positive
    const/4 v0, 0x0
    :join
    invoke-static {v0}, Ljava/lang/Float;->floatToRawIntBits(F)I
    move-result v0
    return v0
.end method

.method public static rawDouble(Z)J
    .locals 2
    if-eqz p0, :negative
    const-wide/16 v0, 0x0
    goto :join
    :negative
    const-wide/high16 v0, -0x8000000000000000L
    :join
    invoke-static {v0, v1}, Ljava/lang/Double;->doubleToRawLongBits(D)J
    move-result-wide v0
    return-wide v0
.end method

.method public static storeFloat(Z)V
    .locals 1
    if-eqz p0, :positive
    const/high16 v0, -0x80000000
    goto :join
    :positive
    const/4 v0, 0x0
    :join
    sput v0, Lfixture/SignedSelections;->staticFloat:F
    return-void
.end method

.method public static storeDouble(Z)V
    .locals 2
    if-eqz p0, :negative
    const-wide/16 v0, 0x0
    goto :join
    :negative
    const-wide/high16 v0, -0x8000000000000000L
    :join
    sput-wide v0, Lfixture/SignedSelections;->staticDouble:D
    return-void
.end method

.method public static selectFloat(Z)F
    .locals 1
    if-eqz p0, :positive
    const/high16 v0, -0x80000000
    return v0
    :positive
    const/4 v0, 0x0
    return v0
.end method

.method public static inspectFloat(FLjava/lang/ArithmeticException;)I
    .locals 1
    invoke-static {p0}, Ljava/lang/Float;->floatToRawIntBits(F)I
    move-result v0
    sput v0, Lfixture/SignedSelections;->lastBits:I
    if-eqz p1, :normal
    throw p1
    :normal
    return v0
.end method

.method public static protectedUse(ZLjava/lang/ArithmeticException;)I
    .locals 2
    if-eqz p0, :positive
    const/high16 v0, -0x80000000
    goto :join
    :positive
    const/4 v0, 0x0
    :join
    invoke-static {v0, p1}, Lfixture/SignedSelections;->inspectFloat(FLjava/lang/ArithmeticException;)I
    move-result v0
    :protected_end
    return v0
    :handler
    move-exception v1
    sput-object v1, Lfixture/SignedSelections;->caught:Ljava/lang/ArithmeticException;
    sget v0, Lfixture/SignedSelections;->lastBits:I
    return v0
    .catch Ljava/lang/ArithmeticException; {:join .. :protected_end} :handler
.end method
