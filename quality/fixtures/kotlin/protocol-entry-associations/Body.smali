.class public final Lfixture/AssociatedBody;
.super Lkotlin/coroutines/jvm/internal/SuspendLambda;
.field public label:I
.field public seed:I
.field public savedSeed:I
.field public savedReply:I

.method public invokeSuspend(Ljava/lang/Object;)Ljava/lang/Object;
    .locals 7
    invoke-static {}, Lkotlin/coroutines/intrinsics/IntrinsicsKt;->getCOROUTINE_SUSPENDED()Ljava/lang/Object;
    move-result-object v0
    iget v1, p0, Lfixture/AssociatedBody;->label:I
    const/4 v2, 0x5
    const/4 v3, 0x2
    if-eqz v1, :initial
    if-eq v1, v2, :resume_first
    if-eq v1, v3, :resume_second
    new-instance v2, Ljava/lang/IllegalStateException;
    const-string v3, "invalid state"
    invoke-direct {v2, v3}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v2

    :initial
    invoke-static {p1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    iget v4, p0, Lfixture/AssociatedBody;->seed:I
    invoke-static {v4}, Lfixture/AssociationEffects;->prepare(I)I
    move-result v5
    iput v4, p0, Lfixture/AssociatedBody;->savedSeed:I
    const/4 v2, 0x5
    iput v2, p0, Lfixture/AssociatedBody;->label:I
    invoke-static {v5, p0}, Lfixture/AssociationEffects;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object p1
    if-eq v0, p1, :suspended
    goto :after_first

    :resume_first
    iget v4, p0, Lfixture/AssociatedBody;->savedSeed:I
    invoke-static {p1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    :after_first
    check-cast p1, Ljava/lang/Number;
    invoke-virtual {p1}, Ljava/lang/Number;->intValue()I
    move-result v5
    invoke-static {v5, v4}, Lfixture/AssociationEffects;->record(II)V
    if-ltz v5, :negative
    iput v4, p0, Lfixture/AssociatedBody;->savedSeed:I
    iput v5, p0, Lfixture/AssociatedBody;->savedReply:I
    const/4 v3, 0x2
    iput v3, p0, Lfixture/AssociatedBody;->label:I
    invoke-static {v5, p0}, Lfixture/AssociationEffects;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object p1
    if-eq p1, v0, :suspended
    goto :after_second

    :resume_second
    iget v4, p0, Lfixture/AssociatedBody;->savedSeed:I
    iget v5, p0, Lfixture/AssociatedBody;->savedReply:I
    invoke-static {p1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    :after_second
    check-cast p1, Ljava/lang/Number;
    invoke-virtual {p1}, Ljava/lang/Number;->intValue()I
    move-result v6
    invoke-static {v6, v4, v5}, Lfixture/AssociationEffects;->observe(III)V
    return-object p1

    :negative
    const/4 v6, -0x1
    invoke-static {v6}, Lkotlin/coroutines/jvm/internal/Boxing;->boxInt(I)Ljava/lang/Integer;
    move-result-object v6
    return-object v6

    :suspended
    return-object v0
.end method
