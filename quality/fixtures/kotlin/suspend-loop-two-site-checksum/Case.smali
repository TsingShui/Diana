.class public final Lfixture/SuspendChecksum;
.super Ljava/lang/Object;

.method public static native load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
.end method

.method public static native record(I)V
.end method

.method public static total(IILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    .locals 12
    instance-of v3, p2, Lfixture/SuspendChecksum$Frame;
    if-eqz v3, :new_frame
    move-object v0, p2
    check-cast v0, Lfixture/SuspendChecksum$Frame;
    iget v3, v0, Lfixture/SuspendChecksum$Frame;->label:I
    const/high16 v10, -0x80000000
    and-int v11, v3, v10
    if-eqz v11, :new_frame
    sub-int/2addr v3, v10
    iput v3, v0, Lfixture/SuspendChecksum$Frame;->label:I
    goto :ready
    :new_frame
    new-instance v0, Lfixture/SuspendChecksum$Frame;
    invoke-direct {v0, p2}, Lfixture/SuspendChecksum$Frame;-><init>(Lkotlin/coroutines/Continuation;)V
    :ready
    iget-object v1, v0, Lfixture/SuspendChecksum$Frame;->result:Ljava/lang/Object;
    invoke-static {}, Lkotlin/coroutines/intrinsics/IntrinsicsKt;->getCOROUTINE_SUSPENDED()Ljava/lang/Object;
    move-result-object v2
    iget v3, v0, Lfixture/SuspendChecksum$Frame;->label:I
    packed-switch v3, :states
    new-instance v10, Ljava/lang/IllegalStateException;
    const-string v11, "call to 'resume' before 'invoke' with coroutine"
    invoke-direct {v10, v11}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v10
    :resume_two
    iget v10, v0, Lfixture/SuspendChecksum$Frame;->I$3:I
    iget v4, v0, Lfixture/SuspendChecksum$Frame;->I$0:I
    iget v5, v0, Lfixture/SuspendChecksum$Frame;->I$1:I
    iget v6, v0, Lfixture/SuspendChecksum$Frame;->I$2:I
    iget v7, v0, Lfixture/SuspendChecksum$Frame;->I$4:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v8, v1
    goto :after_two
    :resume_one
    iget v10, v0, Lfixture/SuspendChecksum$Frame;->I$3:I
    iget v4, v0, Lfixture/SuspendChecksum$Frame;->I$0:I
    iget v5, v0, Lfixture/SuspendChecksum$Frame;->I$1:I
    iget v6, v0, Lfixture/SuspendChecksum$Frame;->I$2:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v7, v1
    goto :after_one
    :initial
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move v4, p0
    move v10, p1
    const/4 v5, 0x0
    const/4 v6, 0x0
    :head
    if-ge v6, v4, :finished
    iput v4, v0, Lfixture/SuspendChecksum$Frame;->I$0:I
    iput v5, v0, Lfixture/SuspendChecksum$Frame;->I$1:I
    iput v6, v0, Lfixture/SuspendChecksum$Frame;->I$2:I
    iput v10, v0, Lfixture/SuspendChecksum$Frame;->I$3:I
    const/4 v3, 0x1
    iput v3, v0, Lfixture/SuspendChecksum$Frame;->label:I
    invoke-static {v6, v0}, Lfixture/SuspendChecksum;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v7
    if-eq v7, v2, :suspended
    :after_one
    check-cast v7, Ljava/lang/Number;
    invoke-virtual {v7}, Ljava/lang/Number;->intValue()I
    move-result v7
    invoke-static {v10}, Lfixture/SuspendChecksum;->record(I)V
    if-ltz v7, :finished
    iput v4, v0, Lfixture/SuspendChecksum$Frame;->I$0:I
    iput v5, v0, Lfixture/SuspendChecksum$Frame;->I$1:I
    iput v6, v0, Lfixture/SuspendChecksum$Frame;->I$2:I
    iput v10, v0, Lfixture/SuspendChecksum$Frame;->I$3:I
    iput v7, v0, Lfixture/SuspendChecksum$Frame;->I$4:I
    add-int/lit8 v9, v7, 0x1
    const/4 v3, 0x2
    iput v3, v0, Lfixture/SuspendChecksum$Frame;->label:I
    invoke-static {v9, v0}, Lfixture/SuspendChecksum;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v8
    if-eq v8, v2, :suspended
    :after_two
    check-cast v8, Ljava/lang/Number;
    invoke-virtual {v8}, Ljava/lang/Number;->intValue()I
    move-result v8
    add-int v9, v7, v8
    add-int/2addr v5, v9
    add-int/2addr v10, v6
    invoke-static {v5}, Lfixture/SuspendChecksum;->record(I)V
    add-int/lit8 v6, v6, 0x1
    goto :head
    :finished
    add-int/2addr v5, v10
    invoke-static {v5}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v5
    return-object v5
    :suspended
    return-object v2
    :states
    .packed-switch 0x0
        :initial
        :resume_one
        :resume_two
    .end packed-switch
.end method
