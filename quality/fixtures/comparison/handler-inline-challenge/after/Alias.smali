.class public Ledge/Alias;
.super Ljava/lang/Object;

.method public static b(I)I
    .locals 1
    invoke-static {p0}, Ljava/lang/Integer;->reverse(I)I
    move-result v0
    xor-int/lit8 v0, v0, 31
    return v0
.end method

.method public static a(I)I
    .locals 1
    invoke-static {p0}, Ljava/lang/Integer;->bitCount(I)I
    move-result v0
    add-int/lit8 v0, v0, 5
    return v0
.end method
