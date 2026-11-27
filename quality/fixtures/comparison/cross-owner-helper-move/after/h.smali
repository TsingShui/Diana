.class public Lyy/h;
.super Ljava/lang/Object;

.method public static marker(I)I
    .locals 1
    invoke-static {p0}, Ljava/lang/Integer;->bitCount(I)I
    move-result v0
    return v0
.end method

.method public static n(I)I
    .locals 1
    xor-int/lit8 v0, p0, 19
    mul-int/lit8 v0, v0, 5
    add-int/lit8 v0, v0, -1
    return v0
.end method
