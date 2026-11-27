.class public Lx/b;
.super Ljava/lang/Object;

.method public static a(I)I
    .locals 1
    xor-int/lit8 v0, p0, 15
    return v0
.end method

.method public static route(I)I
    .locals 1
    invoke-static {p0}, Ljava/lang/Integer;->lowestOneBit(I)I
    move-result v0
    invoke-static {v0}, Lx/b;->a(I)I
    move-result v0
    return v0
.end method
