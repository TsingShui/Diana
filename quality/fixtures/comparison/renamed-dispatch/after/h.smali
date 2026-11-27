.class public Lnext/h;
.super Ljava/lang/Object;

.method public static j(II)I
    .locals 3
    invoke-static {p0, p1}, Ljava/lang/Integer;->rotateLeft(II)I
    move-result v2
    xor-int/lit8 v2, v2, 0x35
    return v2
.end method
