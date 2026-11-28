.class public final Lfixture/MixedBoolean;
.super Ljava/lang/Object;

.method public static mixed()Z
    .locals 1
    invoke-static {}, Lfixture/BooleanEffects;->first()Z
    move-result v0
    if-eqz v0, :fallback
    invoke-static {}, Lfixture/BooleanEffects;->second()Z
    move-result v0
    if-eqz v0, :yes
    invoke-static {}, Lfixture/BooleanEffects;->third()Z
    move-result v0
    if-nez v0, :yes
:fallback
    invoke-static {}, Lfixture/BooleanEffects;->fourth()Z
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

.method public static effectful()Z
    .locals 1
    invoke-static {}, Lfixture/BooleanEffects;->first()Z
    move-result v0
    if-eqz v0, :next
    invoke-static {}, Lfixture/BooleanEffects;->mark()V
    const/4 v0, 0x1
    return v0
:next
    invoke-static {}, Lfixture/BooleanEffects;->second()Z
    move-result v0
    if-nez v0, :no
    const/4 v0, 0x1
    return v0
:no
    const/4 v0, 0x0
    return v0
.end method
