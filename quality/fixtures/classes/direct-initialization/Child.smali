.class public Lfixture/Child;
.super Lfixture/Parent;

.method public constructor <init>(I)V
    .registers 3
    invoke-direct {p0, p1}, Lfixture/Parent;-><init>(I)V
    const/16 v0, 99
    iput v0, p0, Lfixture/Parent;->value:I
    return-void
.end method
