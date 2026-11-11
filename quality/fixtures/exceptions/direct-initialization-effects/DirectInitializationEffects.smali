.class public Lfixture/DirectInitializationEffects;
.super Ljava/lang/Object;
.field public static trace:I
.field public static fault:I
.field public static last:Lfixture/Child;

.method public static event(I)V
    .registers 3
    sget v0, Lfixture/DirectInitializationEffects;->trace:I
    mul-int/lit8 v0, v0, 10
    add-int/2addr v0, p0
    sput v0, Lfixture/DirectInitializationEffects;->trace:I
    sget v0, Lfixture/DirectInitializationEffects;->fault:I
    if-ne v0, p0, :done
    new-instance v0, Ljava/lang/IllegalStateException;
    invoke-direct {v0}, Ljava/lang/IllegalStateException;-><init>()V
    throw v0
    :done
    return-void
.end method

.method public static build(I)Lfixture/Child;
    .registers 5
    :start
    new-instance v0, Lfixture/Child;
    move-object v1, v0
    const/4 v2, 1
    invoke-static {v2}, Lfixture/DirectInitializationEffects;->event(I)V
    if-gez p0, :initialize
    const/4 v2, 3
    invoke-static {v2}, Lfixture/DirectInitializationEffects;->event(I)V
    :initialize
    invoke-direct {v0, p0}, Lfixture/Parent;-><init>(I)V
    const/4 v2, 4
    invoke-static {v2}, Lfixture/DirectInitializationEffects;->event(I)V
    sput-object v1, Lfixture/DirectInitializationEffects;->last:Lfixture/Child;
    :end
    return-object v1
    .catch Ljava/lang/RuntimeException; {:start .. :end} :handler
    :handler
    move-exception v3
    const/4 v2, 5
    invoke-static {v2}, Lfixture/DirectInitializationEffects;->event(I)V
    throw v3
.end method
