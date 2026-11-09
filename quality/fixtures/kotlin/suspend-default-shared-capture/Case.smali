.class public final Lfixture/SuspendCapturedDefault;
.super Ljava/lang/Object;

.method public static native load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
.end method

.method public static native record(I)V
.end method

.method public static choose(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    .locals 12
    instance-of v3, p1, Lfixture/SuspendCapturedDefault$Frame;
    if-eqz v3, :new_frame
    move-object v0, p1
    check-cast v0, Lfixture/SuspendCapturedDefault$Frame;
    iget v3, v0, Lfixture/SuspendCapturedDefault$Frame;->label:I
    const/high16 v10, -0x80000000
    and-int v11, v3, v10
    if-eqz v11, :new_frame
    sub-int/2addr v3, v10
    iput v3, v0, Lfixture/SuspendCapturedDefault$Frame;->label:I
    goto :ready
    :new_frame
    new-instance v0, Lfixture/SuspendCapturedDefault$Frame;
    invoke-direct {v0, p1}, Lfixture/SuspendCapturedDefault$Frame;-><init>(Lkotlin/coroutines/Continuation;)V
    :ready
    iget-object v1, v0, Lfixture/SuspendCapturedDefault$Frame;->result:Ljava/lang/Object;
    invoke-static {}, Lkotlin/coroutines/intrinsics/IntrinsicsKt;->getCOROUTINE_SUSPENDED()Ljava/lang/Object;
    move-result-object v2
    iget v3, v0, Lfixture/SuspendCapturedDefault$Frame;->label:I
    packed-switch v3, :states
    const/16 v9, -0x63
    invoke-static {v9}, Lfixture/SuspendCapturedDefault;->record(I)V
    new-instance v10, Ljava/lang/IllegalStateException;
    const-string v11, "call to 'resume' before 'invoke' with coroutine"
    invoke-direct {v10, v11}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v10
    :resume_two
    iget v7, v0, Lfixture/SuspendCapturedDefault$Frame;->I$1:I
    iget v4, v0, Lfixture/SuspendCapturedDefault$Frame;->I$0:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v5, v1
    goto :after_two
    :resume_one
    iget v7, v0, Lfixture/SuspendCapturedDefault$Frame;->I$1:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v4, v1
    goto :after_one
    :initial
    move v7, p0
    iput v7, v0, Lfixture/SuspendCapturedDefault$Frame;->I$1:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    const/4 v3, 0x1
    iput v3, v0, Lfixture/SuspendCapturedDefault$Frame;->label:I
    invoke-static {p0, v0}, Lfixture/SuspendCapturedDefault;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v4
    if-eq v4, v2, :suspended
    :after_one
    check-cast v4, Ljava/lang/Number;
    invoke-virtual {v4}, Ljava/lang/Number;->intValue()I
    move-result v4
    invoke-static {v4}, Lfixture/SuspendCapturedDefault;->record(I)V
    if-ltz v4, :negative
    iput v4, v0, Lfixture/SuspendCapturedDefault$Frame;->I$0:I
    iput v7, v0, Lfixture/SuspendCapturedDefault$Frame;->I$1:I
    add-int/lit8 v6, v4, 0x1
    const/4 v3, 0x2
    iput v3, v0, Lfixture/SuspendCapturedDefault$Frame;->label:I
    invoke-static {v6, v0}, Lfixture/SuspendCapturedDefault;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v5
    if-eq v5, v2, :suspended
    :after_two
    check-cast v5, Ljava/lang/Number;
    invoke-virtual {v5}, Ljava/lang/Number;->intValue()I
    move-result v5
    invoke-static {v5}, Lfixture/SuspendCapturedDefault;->record(I)V
    add-int/2addr v4, v5
    add-int/2addr v4, v7
    invoke-static {v4}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v4
    return-object v4
    :negative
    const/4 v4, -0x1
    invoke-static {v4}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v4
    return-object v4
    :suspended
    return-object v2
    :states
    .packed-switch 0x0
        :initial
        :resume_one
        :resume_two
    .end packed-switch
.end method
