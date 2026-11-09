.class public final Lfixture/SuspendFinally;
.super Ljava/lang/Object;

.method public static native load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
.end method

.method public static native cleanup(I)V
.end method

.method public static guarded(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    .locals 12
    instance-of v3, p1, Lfixture/SuspendFinally$Frame;
    if-eqz v3, :new_frame
    move-object v0, p1
    check-cast v0, Lfixture/SuspendFinally$Frame;
    iget v3, v0, Lfixture/SuspendFinally$Frame;->label:I
    const/high16 v10, -0x80000000
    and-int v11, v3, v10
    if-eqz v11, :new_frame
    sub-int/2addr v3, v10
    iput v3, v0, Lfixture/SuspendFinally$Frame;->label:I
    goto :ready
    :new_frame
    new-instance v0, Lfixture/SuspendFinally$Frame;
    invoke-direct {v0, p1}, Lfixture/SuspendFinally$Frame;-><init>(Lkotlin/coroutines/Continuation;)V
    :ready
    iget-object v1, v0, Lfixture/SuspendFinally$Frame;->result:Ljava/lang/Object;
    invoke-static {}, Lkotlin/coroutines/intrinsics/IntrinsicsKt;->getCOROUTINE_SUSPENDED()Ljava/lang/Object;
    move-result-object v2
    iget v3, v0, Lfixture/SuspendFinally$Frame;->label:I
    packed-switch v3, :states
    new-instance v10, Ljava/lang/IllegalStateException;
    const-string v11, "call to 'resume' before 'invoke' with coroutine"
    invoke-direct {v10, v11}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v10
    :resume_two
    iget v4, v0, Lfixture/SuspendFinally$Frame;->I$0:I
    iget v5, v0, Lfixture/SuspendFinally$Frame;->I$1:I
    :try_resume_two
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v6, v1
    goto :after_two
    :end_resume_two
    :resume_one
    iget v4, v0, Lfixture/SuspendFinally$Frame;->I$0:I
    :try_resume_one
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v5, v1
    goto :after_one
    :end_resume_one
    :initial
    move v4, p0
    :try_initial
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    iput v4, v0, Lfixture/SuspendFinally$Frame;->I$0:I
    const/4 v3, 0x1
    iput v3, v0, Lfixture/SuspendFinally$Frame;->label:I
    invoke-static {v4, v0}, Lfixture/SuspendFinally;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v5
    if-eq v5, v2, :suspended
    goto :after_one
    :end_initial
    :after_one
    :try_after_one
    check-cast v5, Ljava/lang/Number;
    invoke-virtual {v5}, Ljava/lang/Number;->intValue()I
    move-result v5
    iput v4, v0, Lfixture/SuspendFinally$Frame;->I$0:I
    iput v5, v0, Lfixture/SuspendFinally$Frame;->I$1:I
    add-int/lit8 v7, v5, 0x1
    const/4 v3, 0x2
    iput v3, v0, Lfixture/SuspendFinally$Frame;->label:I
    invoke-static {v7, v0}, Lfixture/SuspendFinally;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v6
    if-eq v6, v2, :suspended
    goto :after_two
    :end_after_one
    :after_two
    :try_after_two
    check-cast v6, Ljava/lang/Number;
    invoke-virtual {v6}, Ljava/lang/Number;->intValue()I
    move-result v6
    add-int/2addr v6, v5
    goto :normal_cleanup
    :end_after_two
    :normal_cleanup
    invoke-static {v4}, Lfixture/SuspendFinally;->cleanup(I)V
    invoke-static {v6}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v6
    return-object v6
    :catch_any
    move-exception v8
    invoke-static {v4}, Lfixture/SuspendFinally;->cleanup(I)V
    throw v8
    :suspended
    return-object v2
    .catchall {:try_resume_two .. :end_resume_two} :catch_any
    .catchall {:try_resume_one .. :end_resume_one} :catch_any
    .catchall {:try_initial .. :end_initial} :catch_any
    .catchall {:try_after_one .. :end_after_one} :catch_any
    .catchall {:try_after_two .. :end_after_two} :catch_any
    :states
    .packed-switch 0x0
        :initial
        :resume_one
        :resume_two
    .end packed-switch
.end method
