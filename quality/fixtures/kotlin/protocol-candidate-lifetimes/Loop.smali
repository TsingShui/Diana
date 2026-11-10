.class public final Lfixture/IntervalLoop;
.super Ljava/lang/Object;

.method public static native prepare(I)I
.end method
.method public static native record(I)V
.end method
.method public static native load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
.end method

.method public static native observe(Lfixture/ReadBox;)V
.end method

.method public static total(IILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    .locals 13
    instance-of v3, p2, Lfixture/IntervalLoop$Frame;
    if-eqz v3, :new_frame
    move-object v0, p2
    check-cast v0, Lfixture/IntervalLoop$Frame;
    iget v3, v0, Lfixture/IntervalLoop$Frame;->label:I
    const/high16 v11, -0x80000000
    and-int v12, v3, v11
    if-eqz v12, :new_frame
    sub-int/2addr v3, v11
    iput v3, v0, Lfixture/IntervalLoop$Frame;->label:I
    goto :ready
    :new_frame
    new-instance v0, Lfixture/IntervalLoop$Frame;
    invoke-direct {v0, p2}, Lfixture/IntervalLoop$Frame;-><init>(Lkotlin/coroutines/Continuation;)V
    :ready
    iget-object v1, v0, Lfixture/IntervalLoop$Frame;->result:Ljava/lang/Object;
    invoke-static {}, Lkotlin/coroutines/intrinsics/IntrinsicsKt;->getCOROUTINE_SUSPENDED()Ljava/lang/Object;
    move-result-object v2
    iget v3, v0, Lfixture/IntervalLoop$Frame;->label:I
    packed-switch v3, :states
    new-instance v11, Ljava/lang/IllegalStateException;
    const-string v12, "call to 'resume' before 'invoke' with coroutine"
    invoke-direct {v11, v12}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v11
    :resumed
    iget v4, v0, Lfixture/IntervalLoop$Frame;->I$0:I
    iget v7, v0, Lfixture/IntervalLoop$Frame;->I$1:I
    iget v6, v0, Lfixture/IntervalLoop$Frame;->I$2:I
    iget v8, v0, Lfixture/IntervalLoop$Frame;->I$3:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v10, v1
    goto :completed
    :initial
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move v4, p0
    const/4 v5, 0x0
    const/4 v6, 0x0
    move v7, p1
    const/4 v8, 0x0
    :head
    if-ge v6, v4, :finished
    invoke-static {v6}, Lfixture/IntervalLoop;->prepare(I)I
    move-result v9
    add-int v8, v5, v7
    invoke-static {v7}, Lfixture/IntervalLoop;->record(I)V
    iput v4, v0, Lfixture/IntervalLoop$Frame;->I$0:I
    iput v7, v0, Lfixture/IntervalLoop$Frame;->I$1:I
    iput v6, v0, Lfixture/IntervalLoop$Frame;->I$2:I
    iput v8, v0, Lfixture/IntervalLoop$Frame;->I$3:I
    const/4 v3, 0x1
    iput v3, v0, Lfixture/IntervalLoop$Frame;->label:I
    invoke-static {v9, v0}, Lfixture/IntervalLoop;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v10
    if-eq v10, v2, :suspended
    goto :completed
    :suspended
    return-object v2
    :completed
    check-cast v10, Ljava/lang/Number;
    invoke-virtual {v10}, Ljava/lang/Number;->intValue()I
    move-result v11
    add-int v5, v8, v11
    new-instance v12, Lfixture/ReadBox;
    invoke-direct {v12, v5, v7}, Lfixture/ReadBox;-><init>(II)V
    invoke-static {v12}, Lfixture/IntervalLoop;->observe(Lfixture/ReadBox;)V
    add-int/2addr v7, v6
    invoke-static {v5}, Lfixture/IntervalLoop;->record(I)V
    add-int/lit8 v6, v6, 0x1
    goto :head
    :finished
    new-instance v12, Lfixture/ReadBox;
    invoke-direct {v12, v8, v7}, Lfixture/ReadBox;-><init>(II)V
    invoke-static {v12}, Lfixture/IntervalLoop;->observe(Lfixture/ReadBox;)V
    invoke-static {v4}, Lfixture/IntervalLoop;->record(I)V
    add-int v11, v5, v7
    invoke-static {v11}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v10
    return-object v10
    :states
    .packed-switch 0x0
        :initial
        :resumed
    .end packed-switch
.end method
