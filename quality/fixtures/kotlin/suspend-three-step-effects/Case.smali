.class public final Lfixture/SuspendThree;
.super Ljava/lang/Object;

.method public static native load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
.end method

.method public static native record(I)V
.end method

.method public static walk(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    .locals 12
    instance-of v3, p1, Lfixture/SuspendThree$Frame;
    if-eqz v3, :new_frame
    move-object v0, p1
    check-cast v0, Lfixture/SuspendThree$Frame;
    iget v3, v0, Lfixture/SuspendThree$Frame;->label:I
    const/high16 v10, -0x80000000
    and-int v11, v3, v10
    if-eqz v11, :new_frame
    sub-int/2addr v3, v10
    iput v3, v0, Lfixture/SuspendThree$Frame;->label:I
    goto :ready
    :new_frame
    new-instance v0, Lfixture/SuspendThree$Frame;
    invoke-direct {v0, p1}, Lfixture/SuspendThree$Frame;-><init>(Lkotlin/coroutines/Continuation;)V
    :ready
    iget-object v1, v0, Lfixture/SuspendThree$Frame;->result:Ljava/lang/Object;
    invoke-static {}, Lkotlin/coroutines/intrinsics/IntrinsicsKt;->getCOROUTINE_SUSPENDED()Ljava/lang/Object;
    move-result-object v2
    iget v3, v0, Lfixture/SuspendThree$Frame;->label:I
    packed-switch v3, :states
    new-instance v10, Ljava/lang/IllegalStateException;
    const-string v11, "call to 'resume' before 'invoke' with coroutine"
    invoke-direct {v10, v11}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v10
    :resume_three
    iget v4, v0, Lfixture/SuspendThree$Frame;->I$0:I
    iget v5, v0, Lfixture/SuspendThree$Frame;->I$1:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v6, v1
    goto :after_three
    :resume_two
    iget v4, v0, Lfixture/SuspendThree$Frame;->I$0:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v5, v1
    goto :after_two
    :resume_one
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v4, v1
    goto :after_one
    :initial
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    const/4 v3, 0x1
    iput v3, v0, Lfixture/SuspendThree$Frame;->label:I
    invoke-static {p0, v0}, Lfixture/SuspendThree;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v4
    if-eq v4, v2, :suspended
    :after_one
    check-cast v4, Ljava/lang/Number;
    invoke-virtual {v4}, Ljava/lang/Number;->intValue()I
    move-result v4
    invoke-static {v4}, Lfixture/SuspendThree;->record(I)V
    if-ltz v4, :negative
    iput v4, v0, Lfixture/SuspendThree$Frame;->I$0:I
    add-int/lit8 v7, v4, 0x1
    const/4 v3, 0x2
    iput v3, v0, Lfixture/SuspendThree$Frame;->label:I
    invoke-static {v7, v0}, Lfixture/SuspendThree;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v5
    if-eq v5, v2, :suspended
    :after_two
    check-cast v5, Ljava/lang/Number;
    invoke-virtual {v5}, Ljava/lang/Number;->intValue()I
    move-result v5
    invoke-static {v5}, Lfixture/SuspendThree;->record(I)V
    if-eqz v5, :zero_second
    iput v4, v0, Lfixture/SuspendThree$Frame;->I$0:I
    iput v5, v0, Lfixture/SuspendThree$Frame;->I$1:I
    add-int v7, v4, v5
    const/4 v3, 0x3
    iput v3, v0, Lfixture/SuspendThree$Frame;->label:I
    invoke-static {v7, v0}, Lfixture/SuspendThree;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v6
    if-eq v6, v2, :suspended
    :after_three
    check-cast v6, Ljava/lang/Number;
    invoke-virtual {v6}, Ljava/lang/Number;->intValue()I
    move-result v6
    invoke-static {v6}, Lfixture/SuspendThree;->record(I)V
    add-int v8, v4, v5
    add-int/2addr v8, v6
    invoke-static {v8}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v8
    return-object v8
    :zero_second
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
        :resume_three
    .end packed-switch
.end method

.method public static normalOnly(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    .locals 12
    instance-of v3, p1, Lfixture/SuspendThree$NormalFrame;
    if-eqz v3, :new_frame
    move-object v0, p1
    check-cast v0, Lfixture/SuspendThree$NormalFrame;
    iget v3, v0, Lfixture/SuspendThree$NormalFrame;->label:I
    const/high16 v10, -0x80000000
    and-int v11, v3, v10
    if-eqz v11, :new_frame
    sub-int/2addr v3, v10
    iput v3, v0, Lfixture/SuspendThree$NormalFrame;->label:I
    goto :ready
    :new_frame
    new-instance v0, Lfixture/SuspendThree$NormalFrame;
    invoke-direct {v0, p1}, Lfixture/SuspendThree$NormalFrame;-><init>(Lkotlin/coroutines/Continuation;)V
    :ready
    iget-object v1, v0, Lfixture/SuspendThree$NormalFrame;->result:Ljava/lang/Object;
    invoke-static {}, Lkotlin/coroutines/intrinsics/IntrinsicsKt;->getCOROUTINE_SUSPENDED()Ljava/lang/Object;
    move-result-object v2
    iget v3, v0, Lfixture/SuspendThree$NormalFrame;->label:I
    packed-switch v3, :states
    new-instance v10, Ljava/lang/IllegalStateException;
    const-string v11, "call to 'resume' before 'invoke' with coroutine"
    invoke-direct {v10, v11}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v10
    :resume_three
    iget v4, v0, Lfixture/SuspendThree$NormalFrame;->I$0:I
    iget v5, v0, Lfixture/SuspendThree$NormalFrame;->I$1:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v6, v1
    goto :after_three
    :resume_two
    iget v4, v0, Lfixture/SuspendThree$NormalFrame;->I$0:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v5, v1
    goto :after_two
    :resume_one
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v4, v1
    goto :after_one
    :initial
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    const/4 v3, 0x1
    iput v3, v0, Lfixture/SuspendThree$NormalFrame;->label:I
    invoke-static {p0, v0}, Lfixture/SuspendThree;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v4
    if-eq v4, v2, :suspended
    const/16 v9, 0x309
    invoke-static {v9}, Lfixture/SuspendThree;->record(I)V
    goto :after_one
    :after_one
    check-cast v4, Ljava/lang/Number;
    invoke-virtual {v4}, Ljava/lang/Number;->intValue()I
    move-result v4
    invoke-static {v4}, Lfixture/SuspendThree;->record(I)V
    if-ltz v4, :negative
    iput v4, v0, Lfixture/SuspendThree$NormalFrame;->I$0:I
    add-int/lit8 v7, v4, 0x1
    const/4 v3, 0x2
    iput v3, v0, Lfixture/SuspendThree$NormalFrame;->label:I
    invoke-static {v7, v0}, Lfixture/SuspendThree;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v5
    if-eq v5, v2, :suspended
    :after_two
    check-cast v5, Ljava/lang/Number;
    invoke-virtual {v5}, Ljava/lang/Number;->intValue()I
    move-result v5
    invoke-static {v5}, Lfixture/SuspendThree;->record(I)V
    if-eqz v5, :zero_second
    iput v4, v0, Lfixture/SuspendThree$NormalFrame;->I$0:I
    iput v5, v0, Lfixture/SuspendThree$NormalFrame;->I$1:I
    add-int v7, v4, v5
    const/4 v3, 0x3
    iput v3, v0, Lfixture/SuspendThree$NormalFrame;->label:I
    invoke-static {v7, v0}, Lfixture/SuspendThree;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v6
    if-eq v6, v2, :suspended
    :after_three
    check-cast v6, Ljava/lang/Number;
    invoke-virtual {v6}, Ljava/lang/Number;->intValue()I
    move-result v6
    invoke-static {v6}, Lfixture/SuspendThree;->record(I)V
    add-int v8, v4, v5
    add-int/2addr v8, v6
    invoke-static {v8}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v8
    return-object v8
    :zero_second
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
        :resume_three
    .end packed-switch
.end method

.method public static privateReturns(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    .locals 12
    instance-of v3, p1, Lfixture/SuspendThree$PrivateFrame;
    if-eqz v3, :new_frame
    move-object v0, p1
    check-cast v0, Lfixture/SuspendThree$PrivateFrame;
    iget v3, v0, Lfixture/SuspendThree$PrivateFrame;->label:I
    const/high16 v10, -0x80000000
    and-int v11, v3, v10
    if-eqz v11, :new_frame
    sub-int/2addr v3, v10
    iput v3, v0, Lfixture/SuspendThree$PrivateFrame;->label:I
    goto :ready
    :new_frame
    new-instance v0, Lfixture/SuspendThree$PrivateFrame;
    invoke-direct {v0, p1}, Lfixture/SuspendThree$PrivateFrame;-><init>(Lkotlin/coroutines/Continuation;)V
    :ready
    iget-object v1, v0, Lfixture/SuspendThree$PrivateFrame;->result:Ljava/lang/Object;
    invoke-static {}, Lkotlin/coroutines/intrinsics/IntrinsicsKt;->getCOROUTINE_SUSPENDED()Ljava/lang/Object;
    move-result-object v2
    iget v3, v0, Lfixture/SuspendThree$PrivateFrame;->label:I
    packed-switch v3, :states
    new-instance v10, Ljava/lang/IllegalStateException;
    const-string v11, "call to 'resume' before 'invoke' with coroutine"
    invoke-direct {v10, v11}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v10
    :resume_three
    iget v4, v0, Lfixture/SuspendThree$PrivateFrame;->I$0:I
    iget v5, v0, Lfixture/SuspendThree$PrivateFrame;->I$1:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v6, v1
    goto :after_three
    :resume_two
    iget v4, v0, Lfixture/SuspendThree$PrivateFrame;->I$0:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v5, v1
    goto :after_two
    :resume_one
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v4, v1
    goto :after_one
    :initial
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    const/4 v3, 0x1
    iput v3, v0, Lfixture/SuspendThree$PrivateFrame;->label:I
    invoke-static {p0, v0}, Lfixture/SuspendThree;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v4
    if-eq v4, v2, :suspended_one
    :after_one
    check-cast v4, Ljava/lang/Number;
    invoke-virtual {v4}, Ljava/lang/Number;->intValue()I
    move-result v4
    invoke-static {v4}, Lfixture/SuspendThree;->record(I)V
    if-ltz v4, :negative
    iput v4, v0, Lfixture/SuspendThree$PrivateFrame;->I$0:I
    add-int/lit8 v7, v4, 0x1
    const/4 v3, 0x2
    iput v3, v0, Lfixture/SuspendThree$PrivateFrame;->label:I
    invoke-static {v7, v0}, Lfixture/SuspendThree;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v5
    if-eq v5, v2, :suspended_two
    :after_two
    check-cast v5, Ljava/lang/Number;
    invoke-virtual {v5}, Ljava/lang/Number;->intValue()I
    move-result v5
    invoke-static {v5}, Lfixture/SuspendThree;->record(I)V
    if-eqz v5, :zero_second
    iput v4, v0, Lfixture/SuspendThree$PrivateFrame;->I$0:I
    iput v5, v0, Lfixture/SuspendThree$PrivateFrame;->I$1:I
    add-int v7, v4, v5
    const/4 v3, 0x3
    iput v3, v0, Lfixture/SuspendThree$PrivateFrame;->label:I
    invoke-static {v7, v0}, Lfixture/SuspendThree;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v6
    if-eq v6, v2, :suspended_three
    :after_three
    check-cast v6, Ljava/lang/Number;
    invoke-virtual {v6}, Ljava/lang/Number;->intValue()I
    move-result v6
    invoke-static {v6}, Lfixture/SuspendThree;->record(I)V
    add-int v8, v4, v5
    add-int/2addr v8, v6
    invoke-static {v8}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v8
    return-object v8
    :zero_second
    invoke-static {v4}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v4
    return-object v4
    :negative
    const/4 v4, -0x1
    invoke-static {v4}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v4
    return-object v4
    :suspended_one
    return-object v2
    :suspended_two
    return-object v2
    :suspended_three
    return-object v2
    :states
    .packed-switch 0x0
        :initial
        :resume_one
        :resume_two
        :resume_three
    .end packed-switch
.end method

.method public static reorderedStates(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    .locals 12
    instance-of v3, p1, Lfixture/SuspendThree$ReorderedFrame;
    if-eqz v3, :new_frame
    move-object v0, p1
    check-cast v0, Lfixture/SuspendThree$ReorderedFrame;
    iget v3, v0, Lfixture/SuspendThree$ReorderedFrame;->label:I
    const/high16 v10, -0x80000000
    and-int v11, v3, v10
    if-eqz v11, :new_frame
    sub-int/2addr v3, v10
    iput v3, v0, Lfixture/SuspendThree$ReorderedFrame;->label:I
    goto :ready
    :new_frame
    new-instance v0, Lfixture/SuspendThree$ReorderedFrame;
    invoke-direct {v0, p1}, Lfixture/SuspendThree$ReorderedFrame;-><init>(Lkotlin/coroutines/Continuation;)V
    :ready
    iget-object v1, v0, Lfixture/SuspendThree$ReorderedFrame;->result:Ljava/lang/Object;
    invoke-static {}, Lkotlin/coroutines/intrinsics/IntrinsicsKt;->getCOROUTINE_SUSPENDED()Ljava/lang/Object;
    move-result-object v2
    iget v3, v0, Lfixture/SuspendThree$ReorderedFrame;->label:I
    sparse-switch v3, :states
    new-instance v10, Ljava/lang/IllegalStateException;
    const-string v11, "call to 'resume' before 'invoke' with coroutine"
    invoke-direct {v10, v11}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v10
    :resume_three
    iget v4, v0, Lfixture/SuspendThree$ReorderedFrame;->I$0:I
    iget v5, v0, Lfixture/SuspendThree$ReorderedFrame;->I$1:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v6, v1
    goto :after_three
    :resume_two
    iget v4, v0, Lfixture/SuspendThree$ReorderedFrame;->I$0:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v5, v1
    goto :after_two
    :resume_one
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v4, v1
    goto :after_one
    :initial
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    const/16 v3, 0x9
    iput v3, v0, Lfixture/SuspendThree$ReorderedFrame;->label:I
    invoke-static {p0, v0}, Lfixture/SuspendThree;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v4
    if-eq v4, v2, :suspended_one
    :after_one
    check-cast v4, Ljava/lang/Number;
    invoke-virtual {v4}, Ljava/lang/Number;->intValue()I
    move-result v4
    invoke-static {v4}, Lfixture/SuspendThree;->record(I)V
    if-ltz v4, :negative
    iput v4, v0, Lfixture/SuspendThree$ReorderedFrame;->I$0:I
    add-int/lit8 v7, v4, 0x1
    const/16 v3, 0x2
    iput v3, v0, Lfixture/SuspendThree$ReorderedFrame;->label:I
    invoke-static {v7, v0}, Lfixture/SuspendThree;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v5
    if-eq v5, v2, :suspended_two
    :after_two
    check-cast v5, Ljava/lang/Number;
    invoke-virtual {v5}, Ljava/lang/Number;->intValue()I
    move-result v5
    invoke-static {v5}, Lfixture/SuspendThree;->record(I)V
    if-eqz v5, :zero_second
    iput v4, v0, Lfixture/SuspendThree$ReorderedFrame;->I$0:I
    iput v5, v0, Lfixture/SuspendThree$ReorderedFrame;->I$1:I
    add-int v7, v4, v5
    const/16 v3, 0x5
    iput v3, v0, Lfixture/SuspendThree$ReorderedFrame;->label:I
    invoke-static {v7, v0}, Lfixture/SuspendThree;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v6
    if-eq v6, v2, :suspended_three
    :after_three
    check-cast v6, Ljava/lang/Number;
    invoke-virtual {v6}, Ljava/lang/Number;->intValue()I
    move-result v6
    invoke-static {v6}, Lfixture/SuspendThree;->record(I)V
    add-int v8, v4, v5
    add-int/2addr v8, v6
    invoke-static {v8}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v8
    return-object v8
    :zero_second
    invoke-static {v4}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v4
    return-object v4
    :negative
    const/4 v4, -0x1
    invoke-static {v4}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v4
    return-object v4
    :suspended_one
    return-object v2
    :suspended_two
    return-object v2
    :suspended_three
    return-object v2
    :states
    .sparse-switch
        0x0 -> :initial
        0x2 -> :resume_two
        0x5 -> :resume_three
        0x9 -> :resume_one
    .end sparse-switch
.end method
