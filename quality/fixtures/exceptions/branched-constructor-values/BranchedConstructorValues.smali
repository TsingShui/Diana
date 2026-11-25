.class public final Lfixture/BranchedConstructorValues;
.super Ljava/lang/Object;

.field private final value:I

.method private constructor <init>(I)V
    .locals 0
    invoke-direct {p0}, Ljava/lang/Object;-><init>()V
    invoke-static {p1}, Lfixture/BranchedConstructorValues;->record(I)V
    iput p1, p0, Lfixture/BranchedConstructorValues;->value:I
    return-void
.end method

.method private constructor <init>(II)V
    .locals 1
    invoke-direct {p0}, Ljava/lang/Object;-><init>()V
    add-int v0, p1, p2
    invoke-static {v0}, Lfixture/BranchedConstructorValues;->record(I)V
    iput v0, p0, Lfixture/BranchedConstructorValues;->value:I
    return-void
.end method

.method private static native record(I)V
.end method

.method private static native consume(II)I
.end method

.method private static native recover(I)I
.end method

.method public static run(ZZ)I
    .locals 5

    new-instance v0, Lfixture/BranchedConstructorValues;
    const/4 v2, 3
    if-eqz p0, :first_pair
    invoke-direct {v0, v2}, Lfixture/BranchedConstructorValues;-><init>(I)V
    goto :first_ready

    :first_pair
    const/4 v3, 4
    invoke-direct {v0, v2, v3}, Lfixture/BranchedConstructorValues;-><init>(II)V
    :first_ready

    :try_start
    new-instance v1, Lfixture/BranchedConstructorValues;
    const/4 v2, 5
    if-eqz p1, :second_pair
    invoke-direct {v1, v2}, Lfixture/BranchedConstructorValues;-><init>(I)V
    goto :both_ready

    :second_pair
    const/4 v3, 6
    invoke-direct {v1, v2, v3}, Lfixture/BranchedConstructorValues;-><init>(II)V
    :both_ready
    iget v2, v0, Lfixture/BranchedConstructorValues;->value:I
    iget v3, v1, Lfixture/BranchedConstructorValues;->value:I
    invoke-static {v2, v3}, Lfixture/BranchedConstructorValues;->consume(II)I
    move-result v4
    :try_end
    return v4

    .catch Ljava/lang/IllegalStateException; {:try_start .. :try_end} :failed

    :failed
    move-exception v4
    iget v2, v0, Lfixture/BranchedConstructorValues;->value:I
    invoke-static {v2}, Lfixture/BranchedConstructorValues;->recover(I)I
    move-result v4
    return v4
.end method
