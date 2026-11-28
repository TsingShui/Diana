.class public final Lfixture/HandlerRethrowRouting;
.super Ljava/lang/Object;

.method private static native setup()V
.end method

.method private static native retry(Ljava/lang/IllegalArgumentException;)Z
.end method

.method private static native schedule()V
.end method

.method private static native waitForAlarm()V
.end method

.method private static native interrupted(Ljava/lang/InterruptedException;)V
.end method

.method private static native cancel()V
.end method

.method private static native sessionFailure(Ljava/lang/Throwable;)V
.end method

.method private static native recover(Ljava/lang/IllegalStateException;)V
.end method

.method public static run()V
    .locals 2

    :setup
    invoke-static {}, Lfixture/HandlerRethrowRouting;->setup()V
    :setup_end
    .catch Ljava/lang/IllegalArgumentException; {:setup .. :setup_end} :retry
    .catchall {:setup .. :setup_end} :session_failed

    :schedule
    invoke-static {}, Lfixture/HandlerRethrowRouting;->schedule()V
    :schedule_end
    .catchall {:schedule .. :schedule_end} :cancel_exceptionally

    :wait
    invoke-static {}, Lfixture/HandlerRethrowRouting;->waitForAlarm()V
    :wait_end
    .catch Ljava/lang/InterruptedException; {:wait .. :wait_end} :interrupted
    .catchall {:wait .. :wait_end} :cancel_exceptionally

    :cancel_normal
    invoke-static {}, Lfixture/HandlerRethrowRouting;->cancel()V
    :cancel_normal_end
    .catchall {:cancel_normal .. :cancel_normal_end} :session_failed
    return-void

    :retry
    move-exception v0
    :decide_retry
    invoke-static {v0}, Lfixture/HandlerRethrowRouting;->retry(Ljava/lang/IllegalArgumentException;)Z
    move-result v1
    :decide_retry_end
    .catchall {:decide_retry .. :decide_retry_end} :session_failed
    if-nez v1, :setup
    return-void

    :interrupted
    move-exception v0
    :mark_interruption
    invoke-static {v0}, Lfixture/HandlerRethrowRouting;->interrupted(Ljava/lang/InterruptedException;)V
    :mark_interruption_end
    .catchall {:mark_interruption .. :mark_interruption_end} :cancel_exceptionally
    goto :cancel_normal

    :cancel_exceptionally
    move-exception v0
    :cleanup
    invoke-static {}, Lfixture/HandlerRethrowRouting;->cancel()V
    throw v0
    :cleanup_end
    .catchall {:cleanup .. :cleanup_end} :session_failed

    :session_failed
    move-exception v0
    invoke-static {v0}, Lfixture/HandlerRethrowRouting;->sessionFailure(Ljava/lang/Throwable;)V
    return-void
.end method

.method public static runWithSecondaryCatch()V
    .locals 2

    :setup
    invoke-static {}, Lfixture/HandlerRethrowRouting;->setup()V
    :setup_end
    .catch Ljava/lang/IllegalArgumentException; {:setup .. :setup_end} :retry
    .catchall {:setup .. :setup_end} :session_failed

    :schedule
    invoke-static {}, Lfixture/HandlerRethrowRouting;->schedule()V
    :schedule_end
    .catchall {:schedule .. :schedule_end} :cancel_exceptionally

    :wait
    invoke-static {}, Lfixture/HandlerRethrowRouting;->waitForAlarm()V
    :wait_end
    .catch Ljava/lang/InterruptedException; {:wait .. :wait_end} :interrupted
    .catchall {:wait .. :wait_end} :cancel_exceptionally

    :cancel_normal
    invoke-static {}, Lfixture/HandlerRethrowRouting;->cancel()V
    :cancel_normal_end
    .catchall {:cancel_normal .. :cancel_normal_end} :session_failed
    return-void

    :retry
    move-exception v0
    :decide_retry
    invoke-static {v0}, Lfixture/HandlerRethrowRouting;->retry(Ljava/lang/IllegalArgumentException;)Z
    move-result v1
    :decide_retry_end
    .catchall {:decide_retry .. :decide_retry_end} :session_failed
    if-nez v1, :setup
    return-void

    :interrupted
    move-exception v0
    :mark_interruption
    invoke-static {v0}, Lfixture/HandlerRethrowRouting;->interrupted(Ljava/lang/InterruptedException;)V
    :mark_interruption_end
    .catch Ljava/lang/IllegalStateException; {:mark_interruption .. :mark_interruption_end} :recover
    .catchall {:mark_interruption .. :mark_interruption_end} :cancel_exceptionally
    goto :cancel_normal

    :recover
    move-exception v0
    :secondary_handler
    invoke-static {v0}, Lfixture/HandlerRethrowRouting;->recover(Ljava/lang/IllegalStateException;)V
    :secondary_end
    .catchall {:secondary_handler .. :secondary_end} :cancel_exceptionally
    goto :cancel_normal

    :cancel_exceptionally
    move-exception v0
    :cleanup
    invoke-static {}, Lfixture/HandlerRethrowRouting;->cancel()V
    throw v0
    :cleanup_end
    .catchall {:cleanup .. :cleanup_end} :session_failed

    :session_failed
    move-exception v0
    invoke-static {v0}, Lfixture/HandlerRethrowRouting;->sessionFailure(Ljava/lang/Throwable;)V
    return-void
.end method
