.class public final Lfixture/ExceptionJoinTypes;
.super Ljava/lang/Object;

.method public static run()V
    .locals 2
    const/4 v0, 0x0

    :loop
    invoke-static {}, Lfixture/ExceptionJoinTypes;->nextStep()I
    move-result v1
    if-eqz v1, :finished

    :attempt
    invoke-static {v1}, Lfixture/ExceptionJoinTypes;->write(I)V
    :attempt_end
    .catch Ljava/io/IOException; {:attempt .. :attempt_end} :io
    .catch Ljava/lang/RuntimeException; {:attempt .. :attempt_end} :runtime
    goto :loop

    :io
    move-exception v0
    invoke-static {}, Lfixture/ExceptionJoinTypes;->seenIo()V
    goto :loop

    :runtime
    move-exception v0
    invoke-static {}, Lfixture/ExceptionJoinTypes;->seenRuntime()V
    goto :loop

    :finished
    if-eqz v0, :success
    instance-of v1, v0, Ljava/io/IOException;
    if-eqz v1, :throw_runtime
    check-cast v0, Ljava/io/IOException;
    throw v0

    :throw_runtime
    check-cast v0, Ljava/lang/RuntimeException;
    throw v0

    :success
    return-void
.end method

.method public static runShared()V
    .locals 2
    const/4 v0, 0x0

    :loop
    invoke-static {}, Lfixture/ExceptionJoinTypes;->nextStep()I
    move-result v1
    if-eqz v1, :finished

    :attempt
    invoke-static {v1}, Lfixture/ExceptionJoinTypes;->write(I)V
    :attempt_end
    .catch Ljava/io/IOException; {:attempt .. :attempt_end} :io
    .catch Ljava/lang/RuntimeException; {:attempt .. :attempt_end} :io
    goto :loop

    :io
    move-exception v0
    invoke-static {}, Lfixture/ExceptionJoinTypes;->seenEither()V
    goto :loop

    :finished
    if-eqz v0, :success
    instance-of v1, v0, Ljava/io/IOException;
    if-eqz v1, :throw_runtime
    check-cast v0, Ljava/io/IOException;
    throw v0

    :throw_runtime
    check-cast v0, Ljava/lang/RuntimeException;
    throw v0

    :success
    return-void
.end method

.method private static native nextStep()I
.end method

.method private static native write(I)V
.end method

.method private static native seenIo()V
.end method

.method private static native seenRuntime()V
.end method

.method private static native seenEither()V
.end method
