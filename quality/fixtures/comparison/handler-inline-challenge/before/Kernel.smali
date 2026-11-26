.class public Ledge/Kernel;
.super Ljava/lang/Object;

.method public static mix(I)I
    .locals 1
    mul-int/lit8 v0, p0, 7
    xor-int/lit8 v0, v0, 0x31
    add-int/lit8 v0, v0, -3
    invoke-static {v0}, Ljava/lang/Integer;->reverseBytes(I)I
    move-result v0
    return v0
.end method
