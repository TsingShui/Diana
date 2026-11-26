.class public Llast/c;
.super Ljava/lang/Object;

.method public static s(II)I
    .locals 1
    if-lt p0, p1, :small
    invoke-static {p0, p1}, Llast/w;->v(II)I
    move-result v0
    return v0
    :small
    const/16 v0, 17
    return v0
.end method

.method public static x(I)I
    .locals 1
    xor-int/lit8 v0, p0, 0x52
    return v0
.end method

.method public static y(I)I
    .locals 1
    xor-int/lit8 v0, p0, 0x52
    return v0
.end method

.method public static n(I)I
    .locals 1
    invoke-static {p0}, Ljava/lang/Integer;->reverse(I)I
    move-result v0
    return v0
.end method

.method public static u(II)I
    .locals 1
    invoke-static {p0, p1}, Llast/w;->v(II)I
    move-result v0
    invoke-static {v0}, Ljava/lang/Integer;->highestOneBit(I)I
    move-result v0
    return v0
.end method
