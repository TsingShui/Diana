.class final LPrefixValue;
.super Ljava/lang/Object;

.field final value:I

.method static constructor <clinit>()V
    .registers 2
    const-string v0, "class-init"
    const/4 v1, 0
    invoke-static {v0, v1, v1}, LConstructorPrefix;->record(Ljava/lang/String;II)V
    sget-boolean v0, LConstructorPrefix;->failClassInit:Z
    if-eqz v0, :initialized
    new-instance v0, Ljava/lang/IllegalStateException;
    invoke-direct {v0}, Ljava/lang/IllegalStateException;-><init>()V
    throw v0
:initialized
    return-void
.end method

.method constructor <init>(II)V
    .registers 4
    invoke-direct {p0}, Ljava/lang/Object;-><init>()V
    const-string v0, "construct"
    invoke-static {v0, p1, p2}, LConstructorPrefix;->record(Ljava/lang/String;II)V
    const/16 v0, 14
    if-ne p2, v0, :constructor_success
    new-instance v0, Ljava/lang/IllegalArgumentException;
    invoke-direct {v0}, Ljava/lang/IllegalArgumentException;-><init>()V
    throw v0
:constructor_success
    mul-int/lit8 v0, p1, 10
    add-int/2addr v0, p2
    iput v0, p0, LPrefixValue;->value:I
    return-void
.end method
