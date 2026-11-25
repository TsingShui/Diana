.class public final Lfixture/RecursiveThrows;
.super Ljava/lang/Object;

.method public static a(Z)V
    .registers 1
    invoke-static {p0}, Lfixture/RecursiveThrows;->b(Z)V
    return-void
.end method

.method public static b(Z)V
    .registers 2
    :begin
    invoke-static {p0}, Lfixture/RecursiveThrows;->c(Z)V
    :end
    .catch Lfixture/ReadFailure; {:begin .. :end} :caught
    return-void
    :caught
    move-exception v0
    return-void
.end method

.method public static c(Z)V
    .registers 1
    if-eqz p0, :read
    const/4 p0, 0
    invoke-static {p0}, Lfixture/RecursiveThrows;->b(Z)V
    return-void
    :read
    invoke-static {}, Lfixture/RecursiveThrows;->zRead()V
    return-void
.end method

.method public static d()V
    .registers 2
    const/4 v0, 0
    :first_start
    invoke-static {v0}, Lfixture/RecursiveThrows;->c(Z)V
    :first_end
    .catch Lfixture/ReadFailure; {:first_start .. :first_end} :io
    goto :again
    :io
    move-exception v1
    :again
    :second_start
    invoke-static {v0}, Lfixture/RecursiveThrows;->c(Z)V
    :second_end
    .catch Ljava/lang/Throwable; {:second_start .. :second_end} :reflection
    return-void
    :reflection
    move-exception v1
    return-void
.end method

.method public static e()V
    .registers 1
    :begin
    invoke-static {}, Lfixture/RecursiveThrows;->d()V
    :end
    .catchall {:begin .. :end} :caught
    return-void
    :caught
    move-exception v0
    return-void
.end method

.method public static native zRead()V
    .annotation system Ldalvik/annotation/Throws;
        value = {Ljava/lang/Throwable;}
    .end annotation
.end method
