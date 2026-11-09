.class public Lfixture/JointCover;
.super Ljava/lang/Object;

.method static native take(I)I
.end method
.method static native pair(II)I
.end method
.method static native mark(I)V
.end method

.method static ordered(I)I
    .registers 4
    if-nez p0, :hit
    const/4 v0, 0x1
    invoke-static {v0}, Lfixture/JointCover;->take(I)I
    move-result v0
    const/4 v1, 0x2
    invoke-static {v1}, Lfixture/JointCover;->take(I)I
    move-result v1
    invoke-static {v0, v1}, Lfixture/JointCover;->pair(II)I
    move-result v2
    if-nez v2, :hit
    const/16 v0, 0x9
    return v0
  :hit
    const/4 v0, 0x7
    return v0
.end method

.method static blocked(I)I
    .registers 4
    if-nez p0, :hit
    const/4 v0, 0x1
    invoke-static {v0}, Lfixture/JointCover;->take(I)I
    move-result v0
    const/4 v1, 0x2
    invoke-static {v1}, Lfixture/JointCover;->take(I)I
    move-result v1
    invoke-static {v0, v1}, Lfixture/JointCover;->pair(II)I
    move-result v2
    invoke-static {v2}, Lfixture/JointCover;->mark(I)V
    if-nez v2, :hit
    const/16 v0, 0x9
    return v0
  :hit
    const/4 v0, 0x7
    return v0
.end method

.method static ordinary(I)I
    .registers 4
    invoke-static {p0}, Lfixture/JointCover;->take(I)I
    move-result v0
    add-int/lit8 v1, p0, 0x1
    invoke-static {v1}, Lfixture/JointCover;->take(I)I
    move-result v1
    invoke-static {v0, v1}, Lfixture/JointCover;->pair(II)I
    move-result v2
    return v2
.end method

.method static reversed(I)I
    .registers 4
    invoke-static {p0}, Lfixture/JointCover;->take(I)I
    move-result v0
    add-int/lit8 v1, p0, 0x1
    invoke-static {v1}, Lfixture/JointCover;->take(I)I
    move-result v1
    invoke-static {v1, v0}, Lfixture/JointCover;->pair(II)I
    move-result v2
    return v2
.end method

.method static constructed(I)Lfixture/CoverBox;
    .registers 5
    new-instance v0, Lfixture/CoverBox;
    invoke-static {p0}, Lfixture/JointCover;->take(I)I
    move-result v1
    add-int/lit8 v2, p0, 0x1
    invoke-static {v2}, Lfixture/JointCover;->take(I)I
    move-result v2
    invoke-direct {v0, v1, v2}, Lfixture/CoverBox;-><init>(II)V
    return-object v0
.end method

.method static tree(I)I
    .registers 5
    invoke-static {p0}, Lfixture/JointCover;->take(I)I
    move-result v0
    add-int/lit8 v1, p0, 0x1
    invoke-static {v1}, Lfixture/JointCover;->take(I)I
    move-result v1
    invoke-static {v0, v1}, Lfixture/JointCover;->pair(II)I
    move-result v0
    add-int/lit8 v1, p0, 0x2
    invoke-static {v1}, Lfixture/JointCover;->take(I)I
    move-result v1
    add-int/lit8 v2, p0, 0x3
    invoke-static {v2}, Lfixture/JointCover;->take(I)I
    move-result v2
    invoke-static {v1, v2}, Lfixture/JointCover;->pair(II)I
    move-result v1
    invoke-static {v0, v1}, Lfixture/JointCover;->pair(II)I
    move-result v0
    return v0
.end method

.method static barrier(I)I
    .registers 4
    invoke-static {p0}, Lfixture/JointCover;->take(I)I
    move-result v0
    invoke-static {v0}, Lfixture/JointCover;->mark(I)V
    add-int/lit8 v1, p0, 0x1
    invoke-static {v1}, Lfixture/JointCover;->take(I)I
    move-result v1
    invoke-static {v0, v1}, Lfixture/JointCover;->pair(II)I
    move-result v2
    return v2
.end method
