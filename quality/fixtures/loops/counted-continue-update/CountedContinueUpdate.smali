.class public final Lfixture/CountedContinueUpdate;
.super Ljava/lang/Object;

.field public static state:I
.field public static events:I
.field public static lastValue:I

.method public static advanceThenStop([IILjava/lang/ArithmeticException;)I
    .registers 5
    const/4 v0, 0x0
:header
    array-length v1, p0
    if-ge v0, v1, :finished
    aget v1, p0, v0
    if-le v1, p1, :observe
    sput v0, Lfixture/CountedContinueUpdate;->state:I
    goto :finished
:observe
    if-eqz v1, :update
    invoke-static {v0, v1, p2}, Lfixture/CountedContinueUpdate;->record(IILjava/lang/ArithmeticException;)V
:update
    add-int/lit8 v0, v0, 0x1
    goto :header
:finished
    sget v0, Lfixture/CountedContinueUpdate;->state:I
    return v0
.end method

.method public static holdOnZero([I)I
    .registers 4
    const/4 v0, 0x0
:header
    array-length v1, p0
    if-ge v0, v1, :finished
    aget v1, p0, v0
    if-nez v1, :not_zero
    const/4 v1, 0x1
    aput v1, p0, v0
    goto :header
:not_zero
    if-gez v1, :observe
    sput v0, Lfixture/CountedContinueUpdate;->state:I
    goto :finished
:observe
    const/4 v2, 0x0
    invoke-static {v0, v1, v2}, Lfixture/CountedContinueUpdate;->record(IILjava/lang/ArithmeticException;)V
    add-int/lit8 v0, v0, 0x1
    goto :header
:finished
    sget v0, Lfixture/CountedContinueUpdate;->state:I
    return v0
.end method

.method public static observeAfterAdvance([ILjava/lang/ArithmeticException;)I
    .registers 4
    const/4 v0, 0x0
:header
    array-length v1, p0
    if-ge v0, v1, :finished
    aget v1, p0, v0
    if-gez v1, :update
    sput v0, Lfixture/CountedContinueUpdate;->state:I
    goto :finished
:update
    add-int/lit8 v0, v0, 0x1
    if-eqz v1, :continue
    invoke-static {v0, v1, p1}, Lfixture/CountedContinueUpdate;->record(IILjava/lang/ArithmeticException;)V
:continue
    goto :header
:finished
    sget v0, Lfixture/CountedContinueUpdate;->state:I
    return v0
.end method

.method private static record(IILjava/lang/ArithmeticException;)V
    .registers 5
    sget v0, Lfixture/CountedContinueUpdate;->events:I
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p0
    add-int/lit8 v0, v0, 0x1
    mul-int/lit8 v0, v0, 0x1f
    add-int/2addr v0, p1
    sput v0, Lfixture/CountedContinueUpdate;->events:I
    sput p1, Lfixture/CountedContinueUpdate;->lastValue:I
    const/4 v1, 0x1
    if-ne p0, v1, :return
    if-eqz p2, :return
    throw p2
:return
    return-void
.end method
