.class public Ledge/Gate;
.super Ljava/lang/Object;

.method public static evaluate(I[II)I
    .locals 2
    mul-int/lit8 v0, p0, 7
    xor-int/lit8 v0, v0, 0x31
    add-int/lit8 v0, v0, -3
    invoke-static {v0}, Ljava/lang/Integer;->reverseBytes(I)I
    move-result v0
    :protected_start
    aget v1, p1, p2
    div-int/2addr v0, v1
    :protected_end
    return v0
    .catch Ljava/lang/RuntimeException; {:protected_start .. :protected_end} :handler
    :handler
    move-exception v1
    const/4 v0, -1
    return v0
.end method
