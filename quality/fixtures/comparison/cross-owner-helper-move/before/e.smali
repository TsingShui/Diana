.class public Lxx/e;
.super Ljava/lang/Object;

.method public static marker(I)I
    .locals 1
    invoke-static {p0}, Ljava/lang/Integer;->reverseBytes(I)I
    move-result v0
    return v0
.end method

.method public static read(I)I
    .locals 1
    invoke-static {p0}, Lxx/e;->m(I)I
    move-result v0
    invoke-static {v0}, Ljava/lang/Integer;->highestOneBit(I)I
    move-result v0
    return v0
.end method

.method public static write(I)I
    .locals 1
    invoke-static {p0}, Lxx/e;->m(I)I
    move-result v0
    invoke-static {v0}, Ljava/lang/Integer;->numberOfLeadingZeros(I)I
    move-result v0
    return v0
.end method

.method public static m(I)I
    .locals 1
    xor-int/lit8 v0, p0, 17
    mul-int/lit8 v0, v0, 5
    add-int/lit8 v0, v0, -1
    return v0
.end method
