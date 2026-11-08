.class public final Lfixture/FinallyOverridesExit;
.super Ljava/lang/Object;

.method public static native open(I)I
.end method

.method public static native read(I)I
.end method

.method public static native close(I)V
.end method

# One explicit finally copy on the normal path and one in the catch-all handler.
# The close call, the return override and the throw override are not inside the
# protected body range, so a close failure escapes and replaces the pending exit
# without being suppressed. Mode bits select the pending throw (1), the return in
# the finally (2) and the throw in the finally (4).
.method public static replaceExit(I)I
    .locals 5
    const/4 v3, 0x0
    invoke-static {v3}, Lfixture/FinallyOverridesExit;->open(I)I
    move-result v0
    :body
    and-int/lit8 v1, p0, 0x1
    if-eqz v1, :after_pending
    new-instance v2, Ljava/lang/IllegalStateException;
    const-string v3, "body"
    invoke-direct {v2, v3}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v2
    :after_pending
    invoke-static {v0}, Lfixture/FinallyOverridesExit;->read(I)I
    move-result v2
    :body_end
    invoke-static {v0}, Lfixture/FinallyOverridesExit;->close(I)V
    and-int/lit8 v1, p0, 0x2
    if-eqz v1, :normal_no_return
    const/4 v2, -0x7
    return v2
    :normal_no_return
    and-int/lit8 v1, p0, 0x4
    if-eqz v1, :normal_no_throw
    new-instance v2, Ljava/lang/IllegalArgumentException;
    const-string v3, "finally"
    invoke-direct {v2, v3}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v2
    :normal_no_throw
    const/4 v2, 0x5
    return v2
    :handler
    move-exception v4
    :handler_close
    invoke-static {v0}, Lfixture/FinallyOverridesExit;->close(I)V
    and-int/lit8 v1, p0, 0x2
    if-eqz v1, :handler_no_return
    const/4 v2, -0x7
    return v2
    :handler_no_return
    and-int/lit8 v1, p0, 0x4
    if-eqz v1, :handler_no_throw
    new-instance v2, Ljava/lang/IllegalArgumentException;
    const-string v3, "finally"
    invoke-direct {v2, v3}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v2
    :handler_no_throw
    throw v4
    .catchall {:body .. :body_end} :handler
.end method
