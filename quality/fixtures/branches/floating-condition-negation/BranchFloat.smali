.class public final Lfixture/BranchFloat;
.super Ljava/lang/Object;
.field public static trace:I

.method public static mark(I)V
    .locals 1
    sget v0, Lfixture/BranchFloat;->trace:I
    mul-int/lit8 v0, v0, 10
    add-int v0, v0, p0
    sput v0, Lfixture/BranchFloat;->trace:I
    return-void
.end method

.method public static fFalls(FF)V
    .locals 2
    cmpg-float v0, p0, p1
    if-gez v0, :small
:big
    const/16 v1, 1
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 2
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 3
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 4
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 5
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    return-void
:small
    const/16 v1, 9
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    return-void
.end method

.method public static dFalls(DD)V
    .locals 2
    cmpl-double v0, p0, p2
    if-lez v0, :small
:big
    const/16 v1, 1
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 2
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 3
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 4
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 5
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    return-void
:small
    const/16 v1, 9
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    return-void
.end method

.method public static fJoinFalls(FF)V
    .locals 2
    cmpg-float v0, p0, p1
    if-gez v0, :small
:big
    const/16 v1, 1
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 2
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 3
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 4
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 5
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    goto :join
:small
    const/16 v1, 9
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
:join
    return-void
.end method

.method public static fJoinBoolean(FFZ)V
    .locals 2
    cmpg-float v0, p0, p1
    if-gez v0, :small
    if-eqz p2, :small
:big
    const/16 v1, 1
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 2
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 3
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 4
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 5
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    goto :join
:small
    const/16 v1, 9
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
:join
    return-void
.end method

.method public static dJoinFalls(DD)V
    .locals 2
    cmpl-double v0, p0, p2
    if-lez v0, :small
:big
    const/16 v1, 1
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 2
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 3
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 4
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    const/16 v1, 5
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
    goto :join
:small
    const/16 v1, 9
    invoke-static {v1}, Lfixture/BranchFloat;->mark(I)V
:join
    return-void
.end method

.method public static iJoinFalls(II)V
    .locals 1
    if-ge p0, p1, :small
    const/16 v0, 1
    invoke-static {v0}, Lfixture/BranchFloat;->mark(I)V
    const/16 v0, 2
    invoke-static {v0}, Lfixture/BranchFloat;->mark(I)V
    const/16 v0, 3
    invoke-static {v0}, Lfixture/BranchFloat;->mark(I)V
    const/16 v0, 4
    invoke-static {v0}, Lfixture/BranchFloat;->mark(I)V
    const/16 v0, 5
    invoke-static {v0}, Lfixture/BranchFloat;->mark(I)V
    goto :join
    :small
    const/16 v0, 9
    invoke-static {v0}, Lfixture/BranchFloat;->mark(I)V
    :join
    return-void
.end method
