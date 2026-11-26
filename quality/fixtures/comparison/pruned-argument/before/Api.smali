.class public Lapp/Api;
.super Ljava/lang/Object;

.method public static first(I)I
    .locals 1
    const/4 v0, 7
    invoke-static {p0, v0}, Lx/u;->k(II)I
    move-result v0
    invoke-static {v0}, Ljava/lang/Integer;->highestOneBit(I)I
    move-result v0
    return v0
.end method

.method public static second(I)I
    .locals 1
    const/4 v0, 7
    invoke-static {p0, v0}, Lx/u;->k(II)I
    move-result v0
    invoke-static {v0}, Ljava/lang/Integer;->reverseBytes(I)I
    move-result v0
    return v0
.end method
