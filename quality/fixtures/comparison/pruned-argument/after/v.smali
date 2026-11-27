.class public Ly/v;
.super Ljava/lang/Object;

.method public static r(I)I
    .locals 2
    invoke-static {p0}, Ljava/lang/Integer;->reverse(I)I
    move-result v0
    xor-int/lit8 v0, v0, 37
    invoke-static {v0}, Ljava/lang/Integer;->bitCount(I)I
    move-result v1
    add-int v0, v0, v1
    return v0
.end method
