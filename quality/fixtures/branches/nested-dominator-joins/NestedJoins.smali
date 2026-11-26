.class public final Lfixture/NestedJoins;
.super Ljava/lang/Object;

.method static native left(I)I
.end method

.method static native right(I)I
.end method

.method static native join(I)I
.end method

.method static native finish(I)I
.end method

.method public static run(II)I
    .locals 1
    and-int/lit8 v0, p0, 0x1
    if-eqz v0, :bypass_0
    and-int/lit8 v0, p0, 0x2
    if-eqz v0, :left_0
    invoke-static {p1}, Lfixture/NestedJoins;->right(I)I
    move-result p1
    goto :join_0
    :left_0
    invoke-static {p1}, Lfixture/NestedJoins;->left(I)I
    move-result p1
    goto :join_0
    :bypass_0
    and-int/lit8 v0, p0, 0x4
    if-nez v0, :finish
    :join_0
    invoke-static {p1}, Lfixture/NestedJoins;->join(I)I
    move-result p1
    ushr-int/lit8 p0, p0, 0x3
    and-int/lit8 v0, p0, 0x1
    if-eqz v0, :bypass_1
    and-int/lit8 v0, p0, 0x2
    if-eqz v0, :left_1
    invoke-static {p1}, Lfixture/NestedJoins;->right(I)I
    move-result p1
    goto :join_1
    :left_1
    invoke-static {p1}, Lfixture/NestedJoins;->left(I)I
    move-result p1
    goto :join_1
    :bypass_1
    and-int/lit8 v0, p0, 0x4
    if-nez v0, :finish
    :join_1
    invoke-static {p1}, Lfixture/NestedJoins;->join(I)I
    move-result p1
    ushr-int/lit8 p0, p0, 0x3
    and-int/lit8 v0, p0, 0x1
    if-eqz v0, :bypass_2
    and-int/lit8 v0, p0, 0x2
    if-eqz v0, :left_2
    invoke-static {p1}, Lfixture/NestedJoins;->right(I)I
    move-result p1
    goto :join_2
    :left_2
    invoke-static {p1}, Lfixture/NestedJoins;->left(I)I
    move-result p1
    goto :join_2
    :bypass_2
    and-int/lit8 v0, p0, 0x4
    if-nez v0, :finish
    :join_2
    invoke-static {p1}, Lfixture/NestedJoins;->join(I)I
    move-result p1
    ushr-int/lit8 p0, p0, 0x3
    and-int/lit8 v0, p0, 0x1
    if-eqz v0, :bypass_3
    and-int/lit8 v0, p0, 0x2
    if-eqz v0, :left_3
    invoke-static {p1}, Lfixture/NestedJoins;->right(I)I
    move-result p1
    goto :join_3
    :left_3
    invoke-static {p1}, Lfixture/NestedJoins;->left(I)I
    move-result p1
    goto :join_3
    :bypass_3
    and-int/lit8 v0, p0, 0x4
    if-nez v0, :finish
    :join_3
    invoke-static {p1}, Lfixture/NestedJoins;->join(I)I
    move-result p1
    ushr-int/lit8 p0, p0, 0x3
    and-int/lit8 v0, p0, 0x1
    if-eqz v0, :bypass_4
    and-int/lit8 v0, p0, 0x2
    if-eqz v0, :left_4
    invoke-static {p1}, Lfixture/NestedJoins;->right(I)I
    move-result p1
    goto :join_4
    :left_4
    invoke-static {p1}, Lfixture/NestedJoins;->left(I)I
    move-result p1
    goto :join_4
    :bypass_4
    and-int/lit8 v0, p0, 0x4
    if-nez v0, :finish
    :join_4
    invoke-static {p1}, Lfixture/NestedJoins;->join(I)I
    move-result p1
    :finish
    invoke-static {p1}, Lfixture/NestedJoins;->finish(I)I
    move-result v0
    return v0
.end method
