.class public Lfixture/ShadowFields;
.super Lfixture/ParentFields;
.field public value:I

.method public constructor <init>()V
    .locals 0
    invoke-direct {p0}, Lfixture/ParentFields;-><init>()V
    return-void
.end method

.method public readOwn()I
    .locals 1
    iget v0, p0, Lfixture/ShadowFields;->value:I
    return v0
.end method

.method public readParent()I
    .locals 1
    iget v0, p0, Lfixture/ParentFields;->value:I
    return v0
.end method

.method public readRoot()I
    .locals 1
    iget v0, p0, Lfixture/RootFields;->value:I
    return v0
.end method

.method public writeParent(I)V
    .locals 0
    iput p1, p0, Lfixture/ParentFields;->value:I
    return-void
.end method

.method public writeRoot(I)V
    .locals 0
    iput p1, p0, Lfixture/RootFields;->value:I
    return-void
.end method

.method public writePeerRoot(Lfixture/ShadowFields;I)I
    .locals 1
    iput p2, p1, Lfixture/RootFields;->value:I
    iget v0, p0, Lfixture/RootFields;->value:I
    return v0
.end method
