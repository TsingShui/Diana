.class public Ledge/Gate;
.super Ljava/lang/Object;

.method public static evaluate(I[II)I
    .locals 2
    invoke-static {p0}, Ledge/Kernel;->mix(I)I
    move-result v0
    :protected_start
    aget v1, p1, p2
    div-int/2addr v0, v1
    :protected_end
    return v0
    .catch Ljava/lang/ArithmeticException; {:protected_start .. :protected_end} :handler
    :handler
    move-exception v1
    const/4 v0, -1
    return v0
.end method
