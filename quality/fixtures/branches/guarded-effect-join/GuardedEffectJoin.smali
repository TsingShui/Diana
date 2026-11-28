.class public final Lfixture/GuardedEffectJoin;
.super Ljava/lang/Object;

.method public static run(ZZ)Ljava/lang/Object;
    .locals 3
    const/4 v0, 0x0

    :start
    if-eqz p0, :use_default
    invoke-static {}, Lfixture/GuardedEffectJoin;->prepare()Ljava/lang/Object;
    move-result-object v1
    goto :join
    :use_default
    move-object v1, v0

    :join
    move-object v2, v1
    invoke-static {p1}, Lfixture/GuardedEffectJoin;->publish(Z)V
    invoke-static {v2}, Lfixture/GuardedEffectJoin;->consume(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v2
    :end
    .catch Ljava/lang/IllegalStateException; {:start .. :end} :caught
    return-object v2

    :caught
    move-exception v1
    invoke-static {}, Lfixture/GuardedEffectJoin;->recover()V
    return-object v0
.end method

.method public static runWithCleanup(ZZ)Ljava/lang/Object;
    .locals 3
    const/4 v0, 0x0

    :start
    if-eqz p0, :use_default
    invoke-static {}, Lfixture/GuardedEffectJoin;->prepare()Ljava/lang/Object;
    move-result-object v1
    goto :join
    :use_default
    move-object v1, v0

    :join
    move-object v2, v1
    invoke-static {p1}, Lfixture/GuardedEffectJoin;->publish(Z)V
    invoke-static {v2}, Lfixture/GuardedEffectJoin;->consume(Ljava/lang/Object;)Ljava/lang/Object;
    move-result-object v2
    :end
    .catch Ljava/lang/IllegalStateException; {:start .. :end} :caught
    .catchall {:start .. :end} :cleanup_failure
    invoke-static {}, Lfixture/GuardedEffectJoin;->cleanup()V
    return-object v2

    :caught
    move-exception v1
    :recovery_start
    invoke-static {}, Lfixture/GuardedEffectJoin;->recover()V
    :recovery_end
    .catchall {:recovery_start .. :recovery_end} :cleanup_failure
    invoke-static {}, Lfixture/GuardedEffectJoin;->cleanup()V
    return-object v0

    :cleanup_failure
    move-exception v1
    invoke-static {}, Lfixture/GuardedEffectJoin;->cleanup()V
    throw v1
.end method

.method private static native prepare()Ljava/lang/Object;
.end method

.method private static native publish(Z)V
.end method

.method private static native consume(Ljava/lang/Object;)Ljava/lang/Object;
.end method

.method private static native recover()V
.end method

.method private static native cleanup()V
.end method
