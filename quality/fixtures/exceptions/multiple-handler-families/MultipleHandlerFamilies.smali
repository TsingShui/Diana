.class public final Lfixture/MultipleHandlerFamilies;
.super Ljava/lang/Object;

.method private static native start()V
.end method

.method private static native retry()V
.end method

.method private static native await()V
.end method

.method private static native mark(ILjava/lang/Throwable;)V
.end method

.method public static run()V
    .locals 2

    :start
    invoke-static {}, Lfixture/MultipleHandlerFamilies;->start()V
    :start_done
    .catch Ljava/lang/IllegalArgumentException; {:start .. :start_done} :left
    .catch Ljava/lang/IndexOutOfBoundsException; {:start .. :start_done} :right
    return-void

    :left
    move-exception v0
    const/4 v1, 0x1
    invoke-static {v1, v0}, Lfixture/MultipleHandlerFamilies;->mark(ILjava/lang/Throwable;)V
    goto :retry

    :right
    move-exception v0
    const/4 v1, 0x2
    invoke-static {v1, v0}, Lfixture/MultipleHandlerFamilies;->mark(ILjava/lang/Throwable;)V
    goto :retry

    :retry
    invoke-static {}, Lfixture/MultipleHandlerFamilies;->retry()V
    :retry_done
    .catch Ljava/lang/UnsupportedOperationException; {:retry .. :retry_done} :recover_left
    .catch Ljava/lang/ArithmeticException; {:retry .. :retry_done} :recover_right
    return-void

    :recover_left
    move-exception v0
    const/4 v1, 0x3
    invoke-static {v1, v0}, Lfixture/MultipleHandlerFamilies;->mark(ILjava/lang/Throwable;)V
    goto :await

    :recover_right
    move-exception v0
    const/4 v1, 0x4
    invoke-static {v1, v0}, Lfixture/MultipleHandlerFamilies;->mark(ILjava/lang/Throwable;)V
    goto :await

    :await
    invoke-static {}, Lfixture/MultipleHandlerFamilies;->await()V
    :await_done
    .catch Ljava/lang/InterruptedException; {:await .. :await_done} :interrupted
    return-void

    :interrupted
    move-exception v0
    const/4 v1, 0x5
    invoke-static {v1, v0}, Lfixture/MultipleHandlerFamilies;->mark(ILjava/lang/Throwable;)V
    return-void
.end method

.method public static runRepeated(I)V
    .locals 2

    if-lez p0, :done

    :start
    invoke-static {}, Lfixture/MultipleHandlerFamilies;->start()V
    :start_done
    .catch Ljava/lang/IllegalArgumentException; {:start .. :start_done} :left
    .catch Ljava/lang/IndexOutOfBoundsException; {:start .. :start_done} :right
    return-void

    :left
    move-exception v0
    const/4 v1, 0x1
    invoke-static {v1, v0}, Lfixture/MultipleHandlerFamilies;->mark(ILjava/lang/Throwable;)V
    goto :retry

    :right
    move-exception v0
    const/4 v1, 0x2
    invoke-static {v1, v0}, Lfixture/MultipleHandlerFamilies;->mark(ILjava/lang/Throwable;)V
    goto :retry

    :retry
    invoke-static {}, Lfixture/MultipleHandlerFamilies;->retry()V
    :retry_done
    .catch Ljava/lang/UnsupportedOperationException; {:retry .. :retry_done} :recover_left
    .catch Ljava/lang/ArithmeticException; {:retry .. :retry_done} :recover_right
    return-void

    :recover_left
    move-exception v0
    const/4 v1, 0x3
    invoke-static {v1, v0}, Lfixture/MultipleHandlerFamilies;->mark(ILjava/lang/Throwable;)V
    goto :await

    :recover_right
    move-exception v0
    const/4 v1, 0x4
    invoke-static {v1, v0}, Lfixture/MultipleHandlerFamilies;->mark(ILjava/lang/Throwable;)V
    goto :await

    :await
    if-lez p0, :done

    :wait
    invoke-static {}, Lfixture/MultipleHandlerFamilies;->await()V
    :await_done
    .catch Ljava/lang/InterruptedException; {:wait .. :await_done} :interrupted
    goto :next

    :interrupted
    move-exception v0
    const/4 v1, 0x5
    invoke-static {v1, v0}, Lfixture/MultipleHandlerFamilies;->mark(ILjava/lang/Throwable;)V
    :next
    add-int/lit8 p0, p0, -0x1
    goto :await

    :done
    return-void
.end method
