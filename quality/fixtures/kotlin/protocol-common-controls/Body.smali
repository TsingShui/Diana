.class public final Lfixture/UnifiedBody;
.super Lkotlin/coroutines/jvm/internal/SuspendLambda;
.field public label:I

.method public invokeSuspend(Ljava/lang/Object;)Ljava/lang/Object;
    .locals 6
    invoke-static {}, Lkotlin/coroutines/intrinsics/IntrinsicsKt;->getCOROUTINE_SUSPENDED()Ljava/lang/Object;
    move-result-object v0
    iget v1, p0, Lfixture/UnifiedBody;->label:I
    const/4 v2, 0x1
    if-eqz v1, :initial
    if-eq v1, v2, :resume
    new-instance v2, Ljava/lang/IllegalStateException;
    const-string v3, "invalid state"
    invoke-direct {v2, v3}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v2
    :initial
    invoke-static {p1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    invoke-static {}, Lfixture/ProtocolEffects;->prepare()I
    move-result v3
    const/4 v2, 0x1
    iput v2, p0, Lfixture/UnifiedBody;->label:I
    invoke-static {v3, p0}, Lfixture/ProtocolEffects;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object p1
    if-ne p1, v0, :after
    return-object v0
    :resume
    invoke-static {p1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    :after
    check-cast p1, Ljava/lang/Number;
    invoke-virtual {p1}, Ljava/lang/Number;->intValue()I
    move-result v3
    invoke-static {v3}, Lfixture/ProtocolEffects;->record(I)V
    if-ltz v3, :negative
    new-instance v5, Lfixture/ReadBox;
    invoke-direct {v5, v3, v3}, Lfixture/ReadBox;-><init>(II)V
    invoke-static {v5}, Lfixture/ProtocolEffects;->observe(Lfixture/ReadBox;)V
    return-object p1
    :negative
    const/4 v3, -0x1
    invoke-static {v3}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v3
    return-object v3
.end method
