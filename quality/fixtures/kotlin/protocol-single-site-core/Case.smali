.class public final Lfixture/OneSite;
.super Ljava/lang/Object;
.method public static native prepare(I)I
.end method
.method public static native load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
.end method
.method public static native record(I)V
.end method
.method public static native observe(Lfixture/ReadBox;)V
.end method
.method public static choose(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    .locals 12
    instance-of v3, p1, Lfixture/OneSite$Frame;
    if-eqz v3, :new_frame
    move-object v0, p1
    check-cast v0, Lfixture/OneSite$Frame;
    iget v3, v0, Lfixture/OneSite$Frame;->label:I
    const/high16 v10, -0x80000000
    and-int v11, v3, v10
    if-eqz v11, :new_frame
    sub-int/2addr v3, v10
    iput v3, v0, Lfixture/OneSite$Frame;->label:I
    goto :ready
    :new_frame
    new-instance v0, Lfixture/OneSite$Frame;
    invoke-direct {v0, p1}, Lfixture/OneSite$Frame;-><init>(Lkotlin/coroutines/Continuation;)V
    :ready
    iget-object v1, v0, Lfixture/OneSite$Frame;->result:Ljava/lang/Object;
    invoke-static {}, Lkotlin/coroutines/intrinsics/IntrinsicsKt;->getCOROUTINE_SUSPENDED()Ljava/lang/Object;
    move-result-object v2
    iget v3, v0, Lfixture/OneSite$Frame;->label:I
    packed-switch v3, :states
    new-instance v10, Ljava/lang/IllegalStateException;
    const-string v11, "call to 'resume' before 'invoke' with coroutine"
    invoke-direct {v10, v11}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v10
    :resumed
    iget v5, v0, Lfixture/OneSite$Frame;->I$0:I
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move-object v4, v1
    goto :after
    :initial
    invoke-static {v1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    move v5, p0
    invoke-static {v5}, Lfixture/OneSite;->prepare(I)I
    move-result v7
    iput v5, v0, Lfixture/OneSite$Frame;->I$0:I
    const/4 v3, 0x1
    iput v3, v0, Lfixture/OneSite$Frame;->label:I
    invoke-static {v7, v0}, Lfixture/OneSite;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v4
    if-eq v4, v2, :suspended
    :after
    check-cast v4, Ljava/lang/Number;
    invoke-virtual {v4}, Ljava/lang/Number;->intValue()I
    move-result v6
    invoke-static {v6}, Lfixture/OneSite;->record(I)V
    if-ltz v6, :negative
    new-instance v9, Lfixture/ReadBox;
    invoke-direct {v9, v6, v5}, Lfixture/ReadBox;-><init>(II)V
    invoke-static {v9}, Lfixture/OneSite;->observe(Lfixture/ReadBox;)V
    add-int v7, v6, v5
    invoke-static {v7}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v7
    return-object v7
    :negative
    const/4 v7, -0x1
    invoke-static {v7}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v7
    return-object v7
    :suspended
    return-object v2
    :states
    .packed-switch 0x0
        :initial
        :resumed
    .end packed-switch
.end method
