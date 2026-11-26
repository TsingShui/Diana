.class public final Lfixture/BoundValues;
.super Ljava/lang/Object;

.method public static native take(FF)Z
.end method

.method public static native done(FF)V
.end method

.method public static native keep(II)V
.end method

.method public static native replacement(Ljava/lang/Throwable;)V
.end method

.method public static native mayThrow()V
.end method

.method public static pair(IIZZ)I
    .locals 3
    if-eqz p2, :keepX
    const/4 p0, 0x0
    :keepX
    if-eqz p3, :keepY
    const/4 p1, 0x0
    :keepY
    if-nez p0, :normal
    if-nez p1, :normal
    const/4 v2, -0x1
    return v2
    :normal
    int-to-float v0, p0
    int-to-float v1, p1
    invoke-static {v0,v1}, Lfixture/BoundValues;->take(FF)Z
    move-result v2
    if-nez v2, :returnSum
    invoke-static {v0,v1}, Lfixture/BoundValues;->done(FF)V
    invoke-static {p0,p1}, Lfixture/BoundValues;->keep(II)V
    :returnSum
    add-int v2, p0, p1
    return v2
.end method

.method public static guarded(IZ)I
    .locals 2
    if-eqz p1, :keep
    const/4 p0, 0
    :keep
    if-nez p0, :use
    const/4 v0, -1
    return v0
    :use
    int-to-float v0, p0
    invoke-static {v0,v0}, Lfixture/BoundValues;->take(FF)Z
    move-result v1
    if-nez v1, :done
    invoke-static {v0,v0}, Lfixture/BoundValues;->done(FF)V
    :done
    return p0
.end method

.method public static caughtDefault(ZZ)I
    .locals 3
    :begin
    invoke-static {}, Lfixture/BoundValues;->mayThrow()V
    :end
    .catch Ljava/lang/RuntimeException; {:begin .. :end} :caught
    const/4 v2, -1
    return v2
    :caught
    move-exception v0
    if-eqz p0, :test
    const/4 v0, 0
    :test
    if-eqz p1, :normal
    invoke-static {v0}, Lfixture/BoundValues;->replacement(Ljava/lang/Throwable;)V
    :normal
    instance-of v1, v0, Ljava/lang/IllegalStateException;
    const/4 v2, 5
    if-eqz p1, :skip
    invoke-static {v1,v2}, Lfixture/BoundValues;->keep(II)V
    :skip
    invoke-static {v1,v2}, Lfixture/BoundValues;->keep(II)V
    invoke-static {v0}, Lfixture/BoundValues;->replacement(Ljava/lang/Throwable;)V
    return v1
.end method

.method public static localConstant(Z)V
    .locals 2
    const/16 v0, 0x25
    int-to-float v1,v0
    if-eqz p0, :skip
    invoke-static {v1,v1}, Lfixture/BoundValues;->done(FF)V
    :skip
    invoke-static {v1,v1}, Lfixture/BoundValues;->done(FF)V
    return-void
.end method
