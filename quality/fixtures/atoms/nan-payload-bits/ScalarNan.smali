.class public final Lfixture/ScalarNan;
.super Ljava/lang/Object;

.field public static stored:D
.field public static lastBits:J
.field public static caught:Ljava/lang/ArithmeticException;

.method public static floatValue()F
    .registers 1
    const v0, 0x7fc01234
    return v0
.end method

.method public static doubleValue()D
    .registers 2
    const-wide v0, -0x7ffffffffa988L
    return-wide v0
.end method

.method public static bits(Z)I
    .registers 2
    if-eqz p0, :negative
    const v0, 0x7fc01234
    goto :observe
    :negative
    const v0, -0x3fbcdf
    :observe
    invoke-static {v0}, Ljava/lang/Float;->floatToRawIntBits(F)I
    move-result v0
    return v0
.end method

.method public static store()V
    .registers 2
    const-wide v0, -0x7ffffffffa988L
    sput-wide v0, Lfixture/ScalarNan;->stored:D
    return-void
.end method

.method public static canonicalFloat()F
    .locals 1
    const/high16 v0, 0x7fc00000
    return v0
.end method

.method public static inspectDouble(DLjava/lang/ArithmeticException;)J
    .locals 2
    invoke-static {p0, p1}, Ljava/lang/Double;->doubleToRawLongBits(D)J
    move-result-wide v0
    sput-wide v0, Lfixture/ScalarNan;->lastBits:J
    if-eqz p2, :normal
    throw p2
    :normal
    return-wide v0
.end method

.method public static protectedBits(ZLjava/lang/ArithmeticException;)J
    .locals 3
    if-eqz p0, :negative
    const-wide v0, 0x7ff8000000009abcL
    goto :join
    :negative
    const-wide v0, -0x7ffffffffa988L
    :join
    invoke-static {v0, v1, p1}, Lfixture/ScalarNan;->inspectDouble(DLjava/lang/ArithmeticException;)J
    move-result-wide v0
    :protected_end
    return-wide v0
    :handler
    move-exception v2
    sput-object v2, Lfixture/ScalarNan;->caught:Ljava/lang/ArithmeticException;
    sget-wide v0, Lfixture/ScalarNan;->lastBits:J
    return-wide v0
    .catch Ljava/lang/ArithmeticException; {:join .. :protected_end} :handler
.end method
