.class public final Lfixture/FinallyReturnCapture;
.super Ljava/lang/Object;

.field private static state:I

.method public static native record(I)V
.end method

.method public static localReturn(I)I
    .locals 5
    const/4 v0, 0x1
    :body
    const/4 v1, 0x1
    if-ne p0, v1, :after_mode1
    const/4 v0, 0x2
    :after_mode1
    const/4 v1, 0x2
    if-ne p0, v1, :after_mode2
    new-instance v1, Ljava/lang/IllegalStateException;
    const-string v2, "body"
    invoke-direct {v1, v2}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v1
    :after_mode2
    move v2, v0
    :body_end
    const/4 v0, 0x3
    invoke-static {v0}, Lfixture/FinallyReturnCapture;->record(I)V
    return v2
    :handler
    move-exception v1
    const/4 v0, 0x3
    invoke-static {v0}, Lfixture/FinallyReturnCapture;->record(I)V
    throw v1
    .catchall {:body .. :body_end} :handler
.end method

.method public static fieldReturn(I)I
    .locals 5
    const/16 v0, 0xa
    sput v0, Lfixture/FinallyReturnCapture;->state:I
    :body
    const/4 v1, 0x1
    if-ne p0, v1, :after_throw
    new-instance v1, Ljava/lang/IllegalStateException;
    const-string v2, "body"
    invoke-direct {v1, v2}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v1
    :after_throw
    sget v2, Lfixture/FinallyReturnCapture;->state:I
    :body_end
    const/16 v0, 0x63
    sput v0, Lfixture/FinallyReturnCapture;->state:I
    invoke-static {v0}, Lfixture/FinallyReturnCapture;->record(I)V
    return v2
    :handler
    move-exception v1
    const/16 v0, 0x63
    sput v0, Lfixture/FinallyReturnCapture;->state:I
    invoke-static {v0}, Lfixture/FinallyReturnCapture;->record(I)V
    throw v1
    .catchall {:body .. :body_end} :handler
.end method
