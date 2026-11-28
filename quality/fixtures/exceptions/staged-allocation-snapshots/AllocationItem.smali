.class final Lfixture/AllocationItem;
.super Ljava/lang/Object;

.method static constructor <clinit>()V
    .locals 0
    invoke-static {}, Lfixture/AllocationStages;->initialized()V
    return-void
.end method

.method constructor <init>(III)V
    .locals 0
    invoke-direct {p0}, Ljava/lang/Object;-><init>()V
    invoke-static {p1, p2, p3}, Lfixture/AllocationStages;->construct(III)V
    return-void
.end method
