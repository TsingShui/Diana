.class public Lnext/m;
.super Ljava/lang/Object;

.method public static f(II)I
    .locals 3
    if-le p0, p1, :small
    invoke-static {p1, p0}, Lnext/h;->j(II)I
    move-result v2
    return v2
    :small
    const/16 v2, 19
    return v2
.end method

.method public static b(I)I
    .locals 3
    xor-int/lit8 v2, p0, 0x52
    return v2
.end method

.method public static d(I)I
    .locals 3
    xor-int/lit8 v2, p0, 0x52
    return v2
.end method

.method public static p(I)I
    .locals 3
    invoke-static {p0}, Ljava/lang/Integer;->bitCount(I)I
    move-result v2
    return v2
.end method

.method public static r(II)I
    .locals 3
    invoke-static {p0, p1}, Lnext/h;->j(II)I
    move-result v2
    invoke-static {v2}, Ljava/lang/Integer;->highestOneBit(I)I
    move-result v2
    return v2
.end method
