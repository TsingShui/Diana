.class public final Lfixture/ArgumentEffects;
.super Ljava/lang/Object;
.method public static native read(II)I
.end method
.method public static native finish(I)I
.end method
.method public static native message(I)Ljava/lang/String;
.end method
.method public static native note(ILjava/lang/Throwable;)V
.end method
.method public static tree(II)Lfixture/Point;
    .locals 8
    const/4 v6, 0x0
    const/4 v7, 0x1
    const/4 v5, 0x2
    :body
    new-instance v0, Lfixture/Point;
    invoke-static {v6, p1}, Lfixture/ArgumentEffects;->read(II)I
    move-result v1
    invoke-static {v7, p1}, Lfixture/ArgumentEffects;->read(II)I
    move-result v2
    add-int v3, v1, v2
    invoke-static {v3}, Lfixture/ArgumentEffects;->finish(I)I
    move-result v3
    invoke-static {v5, p1}, Lfixture/ArgumentEffects;->read(II)I
    move-result v4
    add-int v4, v4, p0
    invoke-direct {v0, v3, v4}, Lfixture/Point;-><init>(II)V
    :body_end
    return-object v0
    :handler
    move-exception v0
    invoke-static {p0, v0}, Lfixture/ArgumentEffects;->note(ILjava/lang/Throwable;)V
    const/4 v0, 0x0
    return-object v0
    .catchall {:body .. :body_end} :handler
.end method
.method public static caughtError(I)V
    .locals 3
    const/4 v2, 0x0
    :body
    new-instance v0, Ljava/lang/IllegalArgumentException;
    invoke-static {v2, p0}, Lfixture/ArgumentEffects;->read(II)I
    move-result v1
    invoke-static {v1}, Lfixture/ArgumentEffects;->message(I)Ljava/lang/String;
    move-result-object v1
    invoke-direct {v0, v1}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v0
    :body_end
    :handler
    move-exception v0
    invoke-static {v2, v0}, Lfixture/ArgumentEffects;->note(ILjava/lang/Throwable;)V
    return-void
    .catchall {:body .. :body_end} :handler
.end method
.method public static reversed(I)Lfixture/Point;
    .locals 4
    new-instance v0, Lfixture/Point;
    const/4 v3, 0x1
    invoke-static {v3, p0}, Lfixture/ArgumentEffects;->read(II)I
    move-result v2
    const/4 v3, 0x0
    invoke-static {v3, p0}, Lfixture/ArgumentEffects;->read(II)I
    move-result v1
    invoke-direct {v0, v1, v2}, Lfixture/Point;-><init>(II)V
    return-object v0
.end method
.method public static shared(I)Lfixture/Point;
    .locals 3
    new-instance v0, Lfixture/Point;
    const/4 v2, 0x0
    invoke-static {v2, p0}, Lfixture/ArgumentEffects;->read(II)I
    move-result v1
    invoke-direct {v0, v1, v1}, Lfixture/Point;-><init>(II)V
    return-object v0
.end method
.method public static interleaved(I)Lfixture/Point;
    .locals 4
    new-instance v0, Lfixture/Point;
    const/4 v3, 0x0
    invoke-static {v3, p0}, Lfixture/ArgumentEffects;->read(II)I
    move-result v1
    const/4 v3, 0x1
    invoke-static {v3, p0}, Lfixture/ArgumentEffects;->read(II)I
    move-result v2
    invoke-static {v1}, Lfixture/ArgumentEffects;->finish(I)I
    move-result v1
    invoke-direct {v0, v1, v2}, Lfixture/Point;-><init>(II)V
    return-object v0
.end method
.method public static differentProtection(I)Lfixture/Point;
    .locals 4
    const/4 v3, 0x0
    :first
    new-instance v0, Lfixture/Point;
    invoke-static {v3, p0}, Lfixture/ArgumentEffects;->read(II)I
    move-result v1
    :first_end
    const/4 v3, 0x1
    :second
    invoke-static {v3, p0}, Lfixture/ArgumentEffects;->read(II)I
    move-result v2
    :second_end
    :init
    invoke-direct {v0, v1, v2}, Lfixture/Point;-><init>(II)V
    :init_end
    return-object v0
    :handler_a
    move-exception v0
    const/16 v3, 0xa
    invoke-static {v3, v0}, Lfixture/ArgumentEffects;->note(ILjava/lang/Throwable;)V
    const/4 v0, 0x0
    return-object v0
    :handler_b
    move-exception v0
    const/16 v3, 0x14
    invoke-static {v3, v0}, Lfixture/ArgumentEffects;->note(ILjava/lang/Throwable;)V
    const/4 v0, 0x0
    return-object v0
    .catchall {:first .. :first_end} :handler_a
    .catchall {:second .. :second_end} :handler_b
    .catchall {:init .. :init_end} :handler_a
.end method
.method public static changingSnapshot(I)I
    .locals 5
    const/4 v4, 0x0
    const/4 v3, 0x0
    :body
    new-instance v0, Lfixture/Point;
    const/4 v4, 0x1
    invoke-static {v3, p0}, Lfixture/ArgumentEffects;->read(II)I
    move-result v1
    invoke-direct {v0, v1, v4}, Lfixture/Point;-><init>(II)V
    :body_end
    return v4
    :handler
    move-exception v2
    invoke-static {v4, v2}, Lfixture/ArgumentEffects;->note(ILjava/lang/Throwable;)V
    return v4
    .catchall {:body .. :body_end} :handler
.end method
