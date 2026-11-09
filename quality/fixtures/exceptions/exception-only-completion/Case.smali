.class public final Lfixture/ExceptionCompletion;
.super Ljava/lang/Object;

.method public static native touch(I)V
.end method
.method public static native outside(I)V
.end method
.method public static native consume(Ljava/lang/Throwable;I)I
.end method
.method public static native finish(I)V
.end method

.method public static typed(ILjava/lang/Throwable;)I
    .registers 4
    :body
    invoke-static {p0}, Lfixture/ExceptionCompletion;->touch(I)V
    throw p1
    :body_end
    :caught
    move-exception v0
    invoke-static {v0, p0}, Lfixture/ExceptionCompletion;->consume(Ljava/lang/Throwable;I)I
    move-result v1
    return v1
    .catch Ljava/lang/IllegalStateException; {:body .. :body_end} :caught
.end method

.method public static nested(ILjava/lang/Throwable;)I
    .registers 5
    :body
    invoke-static {p0}, Lfixture/ExceptionCompletion;->touch(I)V
    throw p1
    :body_end
    :caught
    move-exception v0
    :consume_start
    invoke-static {v0, p0}, Lfixture/ExceptionCompletion;->consume(Ljava/lang/Throwable;I)I
    move-result v1
    :consume_end
    invoke-static {p0}, Lfixture/ExceptionCompletion;->finish(I)V
    return v1
    :all
    move-exception v2
    invoke-static {p0}, Lfixture/ExceptionCompletion;->finish(I)V
    throw v2
    .catch Ljava/lang/IllegalStateException; {:body .. :body_end} :caught
    .catchall {:body .. :body_end} :all
    .catchall {:consume_start .. :consume_end} :all
.end method

.method public static foreign(ILjava/lang/Throwable;)I
    .registers 4
    if-eqz p0, :body
    invoke-static {p0}, Lfixture/ExceptionCompletion;->outside(I)V
    goto :shared_throw
    :body
    invoke-static {p0}, Lfixture/ExceptionCompletion;->touch(I)V
    :shared_throw
    throw p1
    :body_end
    :caught
    move-exception v0
    invoke-static {v0, p0}, Lfixture/ExceptionCompletion;->consume(Ljava/lang/Throwable;I)I
    move-result v1
    return v1
    .catch Ljava/lang/IllegalStateException; {:body .. :body_end} :caught
.end method
