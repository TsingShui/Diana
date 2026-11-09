.class public final Lfixture/ScalarViews;
.super Ljava/lang/Object;

.method public static probe(C)I
    .registers 2
    add-int/lit16 v0, p0, 1000
    return v0
.end method

.method public static probe(I)I
    .registers 2
    add-int/lit16 v0, p0, 2000
    return v0
.end method

.method public static probe(B)I
    .registers 2
    add-int/lit16 v0, p0, 3000
    return v0
.end method

.method public static probe(S)I
    .registers 2
    add-int/lit16 v0, p0, 4000
    return v0
.end method

.method public static probe(Z)I
    .registers 2
    if-eqz p0, :false
    const/16 v0, 5001
    return v0
    :false
    const/16 v0, 5000
    return v0
.end method

.method public static carrier(Ljava/lang/String;)I
    .registers 5
    const/4 v0, 0
    invoke-virtual {p0, v0}, Ljava/lang/String;->charAt(I)C
    move-result v1
    invoke-static {v1}, Lfixture/ScalarViews;->probe(C)I
    move-result v2
    invoke-static {v1}, Lfixture/ScalarViews;->probe(I)I
    move-result v3
    add-int/2addr v2, v3
    return v2
.end method

.method public static array([C)Ljava/lang/String;
    .registers 4
    new-instance v0, Ljava/lang/StringBuilder;
    invoke-direct {v0}, Ljava/lang/StringBuilder;-><init>()V
    const/4 v1, 0
    aget-char v2, p0, v1
    invoke-virtual {v0, v2}, Ljava/lang/StringBuilder;->append(C)Ljava/lang/StringBuilder;
    invoke-virtual {v0, v2}, Ljava/lang/StringBuilder;->append(I)Ljava/lang/StringBuilder;
    invoke-virtual {v0}, Ljava/lang/StringBuilder;->toString()Ljava/lang/String;
    move-result-object v0
    return-object v0
.end method

.method public static literal(Ljava/lang/String;)Ljava/lang/String;
    .registers 3
    const/16 v0, 47
    const/16 v1, 45
    invoke-virtual {p0, v0, v1}, Ljava/lang/String;->replace(CC)Ljava/lang/String;
    move-result-object v0
    return-object v0
.end method

.method public static escaped(Ljava/lang/String;)Ljava/lang/String;
    .registers 3
    const/16 v0, 39
    const v1, 0xd800
    invoke-virtual {p0, v0, v1}, Ljava/lang/String;->replace(CC)Ljava/lang/String;
    move-result-object v0
    return-object v0
.end method

.method public static boundaries()I
    .registers 3
    const v0, 0xffff
    invoke-static {v0}, Lfixture/ScalarViews;->probe(C)I
    move-result v1
    const/16 v0, -128
    invoke-static {v0}, Lfixture/ScalarViews;->probe(B)I
    move-result v2
    add-int/2addr v1, v2
    const v0, -32768
    invoke-static {v0}, Lfixture/ScalarViews;->probe(S)I
    move-result v2
    add-int/2addr v1, v2
    const/4 v0, 1
    invoke-static {v0}, Lfixture/ScalarViews;->probe(Z)I
    move-result v2
    add-int/2addr v1, v2
    const/4 v0, 0
    invoke-static {v0}, Lfixture/ScalarViews;->probe(Z)I
    move-result v2
    add-int/2addr v1, v2
    return v1
.end method

.method public static parameter(C)I
    .registers 2
    invoke-static {p0}, Lfixture/ScalarViews;->probe(C)I
    move-result v0
    return v0
.end method
