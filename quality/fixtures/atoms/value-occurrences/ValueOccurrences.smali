.class public final Lfixture/ValueOccurrences;
.super Ljava/lang/Object;

.field public static visits:I

.method public static tick(I)I
    .registers 3
    sget v0, Lfixture/ValueOccurrences;->visits:I
    add-int/lit8 v0, v0, 0x1
    sput v0, Lfixture/ValueOccurrences;->visits:I
    add-int/lit8 v1, p0, 0x7
    return v1
.end method

.method public static pair(II)I
    .registers 3
    mul-int/lit8 v0, p0, 0xa
    add-int/2addr v0, p1
    return v0
.end method

.method public static identity(Ljava/lang/Object;)Ljava/lang/Object;
    .registers 1
    return-object p0
.end method

.method public static chain(II)I
    .registers 5
    add-int v0, p0, p1
    move v1, v0
    mul-int/lit8 v0, v1, 0x3
    move v1, v0
    return v1
.end method

.method public static effects(I)I
    .registers 4
    invoke-static {p0}, Lfixture/ValueOccurrences;->tick(I)I
    move-result v0
    move v1, v0
    const/4 v2, 0x2
    invoke-static {v1, v2}, Lfixture/ValueOccurrences;->pair(II)I
    move-result v0
    move v1, v0
    return v1
.end method

.method public static shared(I)I
    .registers 3
    invoke-static {p0}, Lfixture/ValueOccurrences;->tick(I)I
    move-result v0
    move v1, v0
    invoke-static {v1, v1}, Lfixture/ValueOccurrences;->pair(II)I
    move-result v0
    return v0
.end method

.method public static repeated(I)I
    .registers 3
    invoke-static {p0}, Lfixture/ValueOccurrences;->tick(I)I
    move-result v0
    move v1, v0
    invoke-static {v1}, Lfixture/ValueOccurrences;->tick(I)I
    move-result v0
    move v1, v0
    return v1
.end method

.method public static bytes()[B
    .registers 3
    const/4 v0, 0x3
    new-array v1, v0, [B
    fill-array-data v1, :payload
    move-object v2, v1
    return-object v2
    :payload
    .array-data 1
        0x1t
        0x2t
        0x3t
    .end array-data
.end method

.method public static checked(Ljava/lang/Object;)I
    .registers 3
    invoke-static {p0}, Lfixture/ValueOccurrences;->identity(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v0
    move-object v1, v0
    check-cast v1, Ljava/lang/String;
    invoke-virtual {v1}, Ljava/lang/String;->length()I
    move-result v0
    return v0
.end method
