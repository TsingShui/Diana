.class public Lfixture/RecoveryWarningMembers;
.super Ljava/lang/Object;

.method public static broken()Ljava/lang/Object;
    .registers 1
    new-instance v0, Ljava/lang/Object;
    return-object v0
.end method

.method public static originalThrow()V
    .registers 2
    new-instance v0, Ljava/lang/UnsupportedOperationException;
    const-string v1, "Method could not be decompiled during semantic recovery: original application message"
    invoke-direct {v0, v1}, Ljava/lang/UnsupportedOperationException;-><init>(Ljava/lang/String;)V
    throw v0
.end method

.method public static sibling()I
    .registers 1
    const/4 v0, 7
    return v0
.end method
