.class public final Lfixture/PredicateWordJoin;
.super Ljava/lang/Object;

.method private static native record(I)V
.end method

.method private static native absent()V
.end method

.method public static select(Ljava/lang/Object;IZ)I
    .locals 2

    instance-of v0, p0, Ljava/lang/String;
    if-eqz p2, :join
    move v0, p1
    :join
    move v1, v0
    invoke-static {v1}, Lfixture/PredicateWordJoin;->record(I)V
    return v1
.end method

.method public static run(Ljava/lang/Object;[Z)I
    .locals 6

    instance-of v0, p0, Ljava/lang/String;
    array-length v3, p1
    const/4 v2, 0x0

    :loop
    if-ge v2, v3, :done
    aget-boolean v4, p1, v2
    if-eqz v4, :publish
    add-int/lit8 v0, v0, 0x1

    :publish
    move v1, v0
    invoke-static {v1}, Lfixture/PredicateWordJoin;->record(I)V
    if-nez v1, :next
    invoke-static {}, Lfixture/PredicateWordJoin;->absent()V

    :next
    move v0, v1
    add-int/lit8 v2, v2, 0x1
    goto :loop

    :done
    return v0
.end method
