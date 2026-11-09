.class public final Lfixture/ValueOccurrenceEffects;
.super Ljava/lang/Object;

.field public static visits:I
.field public static observed:[B

.method public static tick(I)I
    .registers 3
    sget v0, Lfixture/ValueOccurrenceEffects;->visits:I
    add-int/lit8 v0, v0, 0x1
    sput v0, Lfixture/ValueOccurrenceEffects;->visits:I
    const/16 v0, 0xd
    if-ne p0, v0, :ok
    new-instance v0, Ljava/lang/ArithmeticException;
    invoke-direct {v0}, Ljava/lang/ArithmeticException;-><init>()V
    throw v0
    :ok
    add-int/lit8 v1, p0, 0x7
    return v1
.end method

.method public static pair(II)I
    .registers 3
    mul-int/lit8 v0, p0, 0xa
    add-int/2addr v0, p1
    return v0
.end method

.method public static sink([B)V
    .registers 1
    sput-object p0, Lfixture/ValueOccurrenceEffects;->observed:[B
    return-void
.end method

.method public static combined(II)I
    .registers 6
    invoke-static {p0}, Lfixture/ValueOccurrenceEffects;->tick(I)I
    move-result v0
    move v1, v0
    add-int v0, v1, p1
    move v2, v0
    mul-int/lit8 v0, v2, 0x3
    move v3, v0
    invoke-static {v3, v3}, Lfixture/ValueOccurrenceEffects;->pair(II)I
    move-result v0
    move v1, v0
    invoke-static {v1}, Lfixture/ValueOccurrenceEffects;->tick(I)I
    move-result v0
    move v2, v0
    return v2
.end method

.method public static protectedValue(I)I
    .registers 4
    invoke-static {p0}, Lfixture/ValueOccurrenceEffects;->tick(I)I
    move-result v0
    move v1, v0
    :start
    invoke-static {v1}, Lfixture/ValueOccurrenceEffects;->tick(I)I
    move-result v0
    move v2, v0
    return v2
    :end
    .catch Ljava/lang/ArithmeticException; {:start .. :end} :handler
    :handler
    move-exception v0
    return v1
.end method

.method public static twoArrays()[B
    .registers 4
    const/4 v0, 0x3
    new-array v1, v0, [B
    fill-array-data v1, :first
    move-object v2, v1
    invoke-static {v2}, Lfixture/ValueOccurrenceEffects;->sink([B)V
    new-array v1, v0, [B
    fill-array-data v1, :second
    move-object v3, v1
    return-object v3
    :first
    .array-data 1
        0x1t
        0x2t
        0x3t
    .end array-data
    :second
    .array-data 1
        0x4t
        0x5t
        0x6t
    .end array-data
.end method
