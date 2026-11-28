.class public final Lorigin/LateOwner;
.super Ljava/lang/Object;

.field public static value:I

.method static constructor <clinit>()V
    .locals 1
    invoke-static {}, Lcontrol/OwnerEvents;->begin()V
    const/4 v0, 0x5
    sput v0, Loutside/LateOwner;->value:I
    return-void
.end method

.method public static observed()I
    .locals 1
    sget v0, Lorigin/LateOwner;->value:I
    return v0
.end method
