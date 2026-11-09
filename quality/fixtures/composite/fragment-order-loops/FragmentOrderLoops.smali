.class public final Lfixture/FragmentOrderLoops;
.super Ljava/lang/Object;

.method public static nestedSwitch(II)I
    .registers 4
    move v0, p1
    const/4 v1, 0x4
    if-eqz p0, :early
    :repeat
    if-ge v0, v1, :done
    packed-switch v0, :cases
    add-int/lit8 v0, v0, 0x1
    goto :repeat
    :small
    add-int/lit8 v0, v0, 0x1
    goto :repeat
    :two
    add-int/lit8 v0, v0, 0x2
    goto :repeat
    :done
    return v0
    :early
    const/4 v0, 0x7
    return v0
    :cases
    .packed-switch 0x0
        :small
        :small
        :two
    .end packed-switch
.end method
