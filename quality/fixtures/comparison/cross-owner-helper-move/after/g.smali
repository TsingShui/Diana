.class public Lyy/g;
.super Ljava/lang/Object;

.method public static marker(I)I
    .locals 1
    invoke-static {p0}, Ljava/lang/Integer;->reverseBytes(I)I
    move-result v0
    return v0
.end method

.method public static read(I)I
    .locals 1
    invoke-static {p0}, Lyy/h;->n(I)I
    move-result v0
    invoke-static {v0}, Ljava/lang/Integer;->highestOneBit(I)I
    move-result v0
    return v0
.end method

.method public static write(I)I
    .locals 1
    invoke-static {p0}, Lyy/h;->n(I)I
    move-result v0
    invoke-static {v0}, Ljava/lang/Integer;->numberOfLeadingZeros(I)I
    move-result v0
    return v0
.end method
