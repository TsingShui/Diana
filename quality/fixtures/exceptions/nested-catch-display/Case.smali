.class public final Lfixture/LayeredCatch;
.super Ljava/lang/Object;
.method public static native step(II)I
.end method
.method public static native recover(Ljava/lang/Throwable;II)I
.end method
.method public static native finish(I)V
.end method

.method public static layered(I)I
    .registers 6
    const/4 v0, 0
    :body
    invoke-static {v0, p0}, Lfixture/LayeredCatch;->step(II)I
    move-result v1
    :body_end
    invoke-static {p0}, Lfixture/LayeredCatch;->finish(I)V
    return v1
    :inner
    move-exception v2
    :inner_start
    invoke-static {v2, v0, p0}, Lfixture/LayeredCatch;->recover(Ljava/lang/Throwable;II)I
    move-result v1
    :inner_end
    invoke-static {p0}, Lfixture/LayeredCatch;->finish(I)V
    return v1
    :outer
    move-exception v3
    const/16 v0, 9
    :outer_start
    invoke-static {v3, v0, p0}, Lfixture/LayeredCatch;->recover(Ljava/lang/Throwable;II)I
    move-result v1
    :outer_end
    return v1
    :all
    move-exception v4
    invoke-static {p0}, Lfixture/LayeredCatch;->finish(I)V
    throw v4
    .catch Ljava/lang/IllegalStateException; {:body .. :body_end} :inner
    .catch Ljava/lang/IllegalStateException; {:inner_start .. :inner_end} :outer
    .catchall {:body .. :body_end} :all
    .catchall {:inner_start .. :inner_end} :all
    .catchall {:outer_start .. :outer_end} :all
.end method

.method public static catchBody(I)I
    .registers 6
    const/4 v0, 0
    :first
    invoke-static {v0, p0}, Lfixture/LayeredCatch;->step(II)I
    move-result v1
    :first_end
    return v1
    :caught
    move-exception v2
    :recover_start
    invoke-static {v2, v0, p0}, Lfixture/LayeredCatch;->recover(Ljava/lang/Throwable;II)I
    move-result v1
    :recover_end
    const/4 v0, 1
    :second
    invoke-static {v0, p0}, Lfixture/LayeredCatch;->step(II)I
    move-result v3
    add-int/2addr v1, v3
    return v1
    :second_end
    :argument
    move-exception v4
    const/4 v1, -2
    return v1
    .catch Ljava/lang/IllegalStateException; {:first .. :first_end} :caught
    .catch Ljava/lang/IllegalArgumentException; {:second .. :second_end} :argument
.end method

.method public static aliased(I)I
    .registers 4
    const/4 v0, 0
    :body
    invoke-static {v0, p0}, Lfixture/LayeredCatch;->step(II)I
    move-result v1
    :body_end
    return v1
    :same
    move-exception v2
    :state
    invoke-static {v2, v0, p0}, Lfixture/LayeredCatch;->recover(Ljava/lang/Throwable;II)I
    move-result v1
    :state_end
    return v1
    .catch Ljava/lang/IllegalStateException; {:body .. :body_end} :same
    .catch Ljava/lang/IllegalArgumentException; {:body .. :body_end} :same
.end method
