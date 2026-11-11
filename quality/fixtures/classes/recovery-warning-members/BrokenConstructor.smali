.class public Lfixture/BrokenConstructor;
.super Ljava/lang/Object;

.field public static leaked:Ljava/lang/Object;

.method public constructor <init>()V
    .registers 2
    invoke-direct {p0}, Ljava/lang/Object;-><init>()V
    new-instance v0, Ljava/lang/Object;
    sput-object v0, Lfixture/BrokenConstructor;->leaked:Ljava/lang/Object;
    return-void
.end method

.method public static sibling()I
    .registers 1
    const/16 v0, 9
    return v0
.end method
