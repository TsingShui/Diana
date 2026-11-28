.class public final Lfixture/ExceptionLoopIngress;
.super Ljava/lang/Object;

.method private static native next()Ljava/lang/String;
.end method

.method private static native partial(Ljava/lang/String;)Z
.end method

.method private static native append(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;
.end method

.method private static native parse(Ljava/lang/String;)V
.end method

.method public static run()Ljava/lang/String;
    .locals 2

    const/4 v0, 0x0
    goto :read

    :reset
    const/4 v0, 0x0

    :read
    invoke-static {}, Lfixture/ExceptionLoopIngress;->next()Ljava/lang/String;
    move-result-object v1
    if-eqz v1, :done
    if-eqz v0, :single
    invoke-static {v0, v1}, Lfixture/ExceptionLoopIngress;->append(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;
    move-result-object v0
    invoke-static {v0}, Lfixture/ExceptionLoopIngress;->partial(Ljava/lang/String;)Z
    move-result v1
    if-nez v1, :read

    :try_joined
    invoke-static {v0}, Lfixture/ExceptionLoopIngress;->parse(Ljava/lang/String;)V
    :end_joined
    .catch Ljava/lang/IllegalArgumentException; {:try_joined .. :end_joined} :reset
    .catch Ljava/lang/IndexOutOfBoundsException; {:try_joined .. :end_joined} :read
    goto :reset

    :single
    invoke-static {v1}, Lfixture/ExceptionLoopIngress;->partial(Ljava/lang/String;)Z
    move-result v0
    if-eqz v0, :try_single
    move-object v0, v1
    goto :read

    :try_single
    const/4 v0, 0x0
    invoke-static {v1}, Lfixture/ExceptionLoopIngress;->parse(Ljava/lang/String;)V
    :end_single
    .catch Ljava/lang/IllegalArgumentException; {:try_single .. :end_single} :read
    .catch Ljava/lang/IndexOutOfBoundsException; {:try_single .. :end_single} :read
    goto :read

    :done
    return-object v0
.end method
