.class public final LConstructorPrefix;
.super Ljava/lang/Object;

.field static failClassInit:Z
.field private static final events:Ljava/util/List;

.method static constructor <clinit>()V
    .registers 1
    new-instance v0, Ljava/util/ArrayList;
    invoke-direct {v0}, Ljava/util/ArrayList;-><init>()V
    sput-object v0, LConstructorPrefix;->events:Ljava/util/List;
    return-void
.end method

.method public static run(IZ)I
    .registers 6
    invoke-static {p0}, LConstructorPrefix;->prepareFirst(I)I
    move-result v0
    invoke-static {v0, p1}, LConstructorPrefix;->prepareSecond(IZ)I
    move-result v1
    new-instance v2, LPrefixValue;
    add-int/lit8 v1, v1, 1
    mul-int/lit8 v3, p0, 2
    invoke-direct {v2, v1, v3}, LPrefixValue;-><init>(II)V
    iget v0, v2, LPrefixValue;->value:I
    return v0
.end method

.method private static prepareFirst(I)I
    .registers 3
    const-string v0, "first"
    const/4 v1, 0
    invoke-static {v0, p0, v1}, LConstructorPrefix;->record(Ljava/lang/String;II)V
    add-int/lit8 v0, p0, 1
    return v0
.end method

.method private static prepareSecond(IZ)I
    .registers 3
    const-string v0, "second"
    invoke-static {v0, p0, p1}, LConstructorPrefix;->record(Ljava/lang/String;II)V
    if-eqz p1, :ready
    new-instance v0, Ljava/lang/IllegalStateException;
    invoke-direct {v0}, Ljava/lang/IllegalStateException;-><init>()V
    throw v0
:ready
    add-int/lit8 v0, p0, 2
    return v0
.end method

.method static record(Ljava/lang/String;II)V
    .registers 6
    new-instance v0, Ljava/lang/StringBuilder;
    invoke-direct {v0}, Ljava/lang/StringBuilder;-><init>()V
    invoke-virtual {v0, p0}, Ljava/lang/StringBuilder;->append(Ljava/lang/String;)Ljava/lang/StringBuilder;
    const/16 v1, 0x3a
    invoke-virtual {v0, v1}, Ljava/lang/StringBuilder;->append(C)Ljava/lang/StringBuilder;
    invoke-virtual {v0, p1}, Ljava/lang/StringBuilder;->append(I)Ljava/lang/StringBuilder;
    invoke-virtual {v0, v1}, Ljava/lang/StringBuilder;->append(C)Ljava/lang/StringBuilder;
    invoke-virtual {v0, p2}, Ljava/lang/StringBuilder;->append(I)Ljava/lang/StringBuilder;
    invoke-virtual {v0}, Ljava/lang/StringBuilder;->toString()Ljava/lang/String;
    move-result-object v2
    sget-object v1, LConstructorPrefix;->events:Ljava/util/List;
    invoke-interface {v1, v2}, Ljava/util/List;->add(Ljava/lang/Object;)Z
    return-void
.end method
