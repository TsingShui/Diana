.class public final Lfixture/AssociatedPublications;
.super Lkotlin/coroutines/jvm/internal/SuspendLambda;
.field public label:I

.method public invokeSuspend(Ljava/lang/Object;)Ljava/lang/Object;
    .locals 4
    invoke-static {}, Lkotlin/coroutines/intrinsics/IntrinsicsKt;->getCOROUTINE_SUSPENDED()Ljava/lang/Object;
    move-result-object v0
    iget v1, p0, Lfixture/AssociatedPublications;->label:I
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
    const/4 v2, 0x5
    iput v2, p0, Lfixture/AssociatedPublications;->label:I
    invoke-static {v2, p0}, Lfixture/AssociationEffects;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object p1
    if-ne p1, v0, :after_first
    :suspended
    return-object v0
    :resume_first
    invoke-static {p1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    goto :after_first
    :resume_second
    invoke-static {p1}, Lkotlin/ResultKt;->throwOnFailure(Ljava/lang/Object;)V
    goto :after_second
    :after_first
    check-cast p1, Ljava/lang/Number;
    invoke-virtual {p1}, Ljava/lang/Number;->intValue()I
    move-result v2
    const/4 v3, 0x2
    iput v3, p0, Lfixture/AssociatedPublications;->label:I
    invoke-static {v2, p0}, Lfixture/AssociationEffects;->load(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object p1
    if-eq p1, v0, :suspended
    :after_second
    return-object p1
.end method
