.class public final Lfixture/GuardedBoolean;
.super Ljava/lang/Object;

.method public static allowed()Z
    .locals 1
    invoke-static {}, Lfixture/BooleanEffects;->first()Z
    move-result v0
    if-nez v0, :yes
    invoke-static {}, Lfixture/BooleanEffects;->second()Z
    move-result v0
    if-nez v0, :no
    goto :yes
:no
    const/4 v0, 0x0
    return v0
:yes
    const/4 v0, 0x1
    return v0
.end method
