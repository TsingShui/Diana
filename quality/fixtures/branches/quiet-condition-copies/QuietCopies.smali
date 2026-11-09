.class public final Lfixture/QuietCopies;
.super Ljava/lang/Object;

.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/QuietCopies;->trace:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    sput v0, Lfixture/QuietCopies;->trace:I
    return-void
.end method

.method public static seed(I)I
    .locals 1
    add-int/lit8 v0, p0, 0x1e
    invoke-static {v0}, Lfixture/QuietCopies;->mark(I)V
    add-int/lit8 v0, p0, 0x1
    return v0
.end method

.method public static pair(IIII)I
    .locals 2
    invoke-static {p0}, Lfixture/QuietCopies;->seed(I)I
    move-result v0
    invoke-static {v0}, Lfixture/QuietCopies;->mark(I)V
    if-nez p1, :change
    if-eqz p2, :first_join
    :change
    add-int/lit8 v0, v0, 0x7
    :first_join
    invoke-static {v0}, Lfixture/QuietCopies;->mark(I)V
    if-nez p2, :choose
    if-eqz p3, :second_join
    :choose
    const/16 p0, 0x28
    :second_join
    invoke-static {p0}, Lfixture/QuietCopies;->mark(I)V
    add-int v1, v0, p0
    return v1
.end method

.method public static quiet(IIII)I
    .locals 2
    invoke-static {p0}, Lfixture/QuietCopies;->seed(I)I
    move-result v0
    const/4 v0, 0x0
    invoke-static {v0}, Lfixture/QuietCopies;->mark(I)V
    if-nez p1, :change
    if-eqz p2, :first_join
    :change
    add-int/lit8 v0, v0, 0x7
    :first_join
    invoke-static {v0}, Lfixture/QuietCopies;->mark(I)V
    if-nez p2, :choose
    if-eqz p3, :second_join
    :choose
    const/16 p0, 0x28
    :second_join
    invoke-static {p0}, Lfixture/QuietCopies;->mark(I)V
    add-int v1, v0, p0
    return v1
.end method
