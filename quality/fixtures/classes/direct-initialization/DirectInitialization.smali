.class public Lfixture/DirectInitialization;
.super Ljava/lang/Object;

.method public static direct(I)Lfixture/Child;
    .registers 2
    new-instance v0, Lfixture/Child;
    invoke-direct {v0, p0}, Lfixture/Parent;-><init>(I)V
    return-object v0
.end method

.method public static ordinary(I)Lfixture/Child;
    .registers 2
    new-instance v0, Lfixture/Child;
    invoke-direct {v0, p0}, Lfixture/Child;-><init>(I)V
    return-object v0
.end method

.method public static broken()Lfixture/Child;
    .registers 1
    new-instance v0, Lfixture/Child;
    return-object v0
.end method
