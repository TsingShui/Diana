.class public final Lfixture/SuspendThree$NormalFrame;
.super Lkotlin/coroutines/jvm/internal/ContinuationImpl;
.field public label:I
.field public result:Ljava/lang/Object;
.field public I$0:I
.field public I$1:I
.method public constructor <init>(Lkotlin/coroutines/Continuation;)V
    .locals 0
    invoke-direct {p0, p1}, Lkotlin/coroutines/jvm/internal/ContinuationImpl;-><init>(Lkotlin/coroutines/Continuation;)V
    return-void
.end method
.method public invokeSuspend(Ljava/lang/Object;)Ljava/lang/Object;
    .locals 2
    iput-object p1, p0, Lfixture/SuspendThree$NormalFrame;->result:Ljava/lang/Object;
    iget v0, p0, Lfixture/SuspendThree$NormalFrame;->label:I
    const/high16 v1, -0x80000000
    or-int/2addr v0, v1
    iput v0, p0, Lfixture/SuspendThree$NormalFrame;->label:I
    const/4 v0, 0x0
    invoke-static {v0, p0}, Lfixture/SuspendThree;->normalOnly(ILkotlin/coroutines/Continuation;)Ljava/lang/Object;
    move-result-object v0
    return-object v0
.end method
