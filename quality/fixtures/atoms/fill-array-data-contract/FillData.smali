.class public final Lfixture/FillData;
.super Ljava/lang/Object;

.method public static fillTwo([I)V
    .registers 1
    fill-array-data p0, :values
    return-void
    :values
    .array-data 4
        0xb
        0x16
    .end array-data
.end method

.method public static fillEmpty([I)V
    .registers 1
    fill-array-data p0, :values
    return-void
    :values
    .array-data 4
    .end array-data
.end method

.method public static fillFloat([F)V
    .registers 1
    fill-array-data p0, :values
    return-void
    :values
    .array-data 4
        0x3fc00000
        -0x80000000
    .end array-data
.end method

.method public static receiver()[I
    .registers 1
    sget v0, Lfixture/FillData;->trace:I
    add-int/lit8 v0, v0, 0x1
    sput v0, Lfixture/FillData;->trace:I
    sget-object v0, Lfixture/FillData;->shared:[I
    return-object v0
.end method

.method public static fillFromReceiver()V
    .registers 1
    invoke-static {}, Lfixture/FillData;->receiver()[I
    move-result-object v0
    fill-array-data v0, :values
    return-void
    :values
    .array-data 4
        0xb
        0x16
    .end array-data
.end method

.field public static trace:I
.field public static shared:[I

.field public static caught:Ljava/lang/ArithmeticException;

.method public static receiverMaybe([ILjava/lang/ArithmeticException;)[I
    .locals 1
    sget v0, Lfixture/FillData;->trace:I
    add-int/lit8 v0, v0, 0x1
    sput v0, Lfixture/FillData;->trace:I
    if-eqz p1, :success
    throw p1
    :success
    return-object p0
.end method

.method public static protectedFill([ILjava/lang/ArithmeticException;)I
    .locals 2
    :attempt
    invoke-static {p0, p1}, Lfixture/FillData;->receiverMaybe([ILjava/lang/ArithmeticException;)[I
    move-result-object v0
    fill-array-data v0, :values
    :attempt_end
    const/4 v1, 0x1
    return v1
    :bounds
    move-exception v1
    const/4 v1, 0x0
    aget v0, p0, v1
    return v0
    :receiver_fault
    move-exception v1
    sput-object v1, Lfixture/FillData;->caught:Ljava/lang/ArithmeticException;
    const/4 v1, -0x1
    return v1
    .catch Ljava/lang/ArrayIndexOutOfBoundsException; {:attempt .. :attempt_end} :bounds
    .catch Ljava/lang/ArithmeticException; {:attempt .. :attempt_end} :receiver_fault
    :values
    .array-data 4
        0xb
        0x16
    .end array-data
.end method

.method public static fillFloatBits([F)V
    .registers 1
    fill-array-data p0, :values
    return-void
    :values
    .array-data 4
        0x0
        -0x80000000
        0x7fc12345
    .end array-data
.end method
