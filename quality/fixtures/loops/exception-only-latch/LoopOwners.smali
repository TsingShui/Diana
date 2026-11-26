.class public final Lfixture/LoopOwners;
.super Ljava/lang/Object;

.method public static native choose(II)I
.end method
.method public static native first(I)I
.end method
.method public static native second(I)I
.end method
.method public static native alternate(I)I
.end method
.method public static native stop(I)I
.end method
.method public static native retry(I)V
.end method
.method public static native empty(I)I
.end method

.method public static dispatch(II)I
    .locals 1
    :header
    if-lez p0, :empty
    :protected_start
    invoke-static {p0,p1}, Lfixture/LoopOwners;->choose(II)I
    move-result v0
    packed-switch v0, :options
    invoke-static {p0}, Lfixture/LoopOwners;->stop(I)I
    move-result v0
    return v0
    :paired
    invoke-static {p0}, Lfixture/LoopOwners;->first(I)I
    move-result v0
    invoke-static {v0}, Lfixture/LoopOwners;->second(I)I
    move-result v0
    return v0
    :alternate
    invoke-static {p0}, Lfixture/LoopOwners;->alternate(I)I
    move-result v0
    return v0
    :protected_end
    .catch Ljava/lang/IllegalStateException; {:protected_start .. :protected_end} :failure

    :failure
    move-exception v0
    invoke-static {p0}, Lfixture/LoopOwners;->retry(I)V
    add-int/lit8 p0, p0, -1
    goto :header
    :empty
    invoke-static {p0}, Lfixture/LoopOwners;->empty(I)I
    move-result v0
    return v0
    :options
    .packed-switch 0x0
        :paired
        :paired
        :alternate
    .end packed-switch
.end method
