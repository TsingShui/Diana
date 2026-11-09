.class public final Lfixture/SharedTails;
.super Ljava/lang/Object;
.field public static trace:I

.method public static mark(I)V
    .locals 2
    sget v0, Lfixture/SharedTails;->trace:I
    const/16 v1, 31
    mul-int/2addr v0, v1
    add-int/2addr v0, p0
    sput v0, Lfixture/SharedTails;->trace:I
    return-void
.end method

.method public static read(II)I
    .locals 3
    add-int/lit16 v0, p0, 100
    invoke-static {v0}, Lfixture/SharedTails;->mark(I)V
    const/4 v0, 1
    if-ne p1, v0, :read_other
    rem-int/lit8 v0, p0, 10
    if-nez v0, :read_other
    new-instance v0, Ljava/lang/IllegalArgumentException;
    const-string v1, "read"
    invoke-direct {v0, v1}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v0
  :read_other
    const/4 v0, 2
    if-eq p1, v0, :read_error_key
    const/4 v0, 4
    if-ne p1, v0, :read_value
  :read_error_key
    const/4 v0, 1
    if-ne p0, v0, :read_value
    new-instance v0, Ljava/lang/AssertionError;
    const-string v1, "read"
    invoke-direct {v0, v1}, Ljava/lang/AssertionError;-><init>(Ljava/lang/Object;)V
    throw v0
  :read_value
    add-int/lit8 v0, p0, 1
    return v0
.end method

.method public static cleanup(I)V
    .locals 2
    const/16 v0, 70
    invoke-static {v0}, Lfixture/SharedTails;->mark(I)V
    const/4 v0, 3
    if-eq p0, v0, :cleanup_fail
    const/4 v0, 4
    if-ne p0, v0, :cleanup_done
  :cleanup_fail
    new-instance v0, Ljava/lang/IllegalStateException;
    const-string v1, "cleanup"
    invoke-direct {v0, v1}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v0
  :cleanup_done
    return-void
.end method

.method public static route(IIIII)I
    .locals 10
    const/4 v0, 0
    const/4 v1, 0
  :outer_start
  :outer_head
    if-ge v0, p0, :normal_cleanup
    move v3, p3
    packed-switch p1, :switch_data
  :default_exit
    add-int/lit8 v6, v0, 90
    invoke-static {v6}, Lfixture/SharedTails;->mark(I)V
    goto :normal_cleanup
  :case_zero
    add-int/lit8 v6, v0, 10
    invoke-static {v6}, Lfixture/SharedTails;->mark(I)V
    add-int/lit8 v1, v1, 1
    goto :tail_a
  :case_one
    const/4 v2, 0
  :inner_head
    const/4 v5, 2
    if-ge v2, v5, :tail_a
    mul-int/lit8 v6, v0, 10
    add-int/2addr v6, v2
  :read_start
    invoke-static {v6, p4}, Lfixture/SharedTails;->read(II)I
    move-result v4
    add-int/2addr v1, v4
  :read_end
    goto :after_read
  :read_bad
    move-exception v7
    add-int/lit8 v6, v2, 30
    invoke-static {v6}, Lfixture/SharedTails;->mark(I)V
    add-int/lit8 v1, v1, 5
  :after_read
    if-eq v2, p2, :early_carry
    add-int/lit8 v2, v2, 1
    goto :inner_head
  :early_carry
    move v3, v1
    goto :tail_b
  :case_two
    add-int/lit8 v6, v0, 20
    invoke-static {v6}, Lfixture/SharedTails;->mark(I)V
    add-int v3, p3, v0
    goto :tail_b
  :tail_a
    add-int/lit8 v1, v1, 3
    move v3, v1
  :tail_b
    move v1, v3
    add-int/lit8 v6, v0, 50
    invoke-static {v6}, Lfixture/SharedTails;->mark(I)V
    add-int/lit8 v0, v0, 1
    goto :outer_head
  :outer_end
    .catch Ljava/lang/IllegalArgumentException; {:read_start .. :read_end} :read_bad
    .catchall {:outer_start .. :outer_end} :outer_all
  :normal_cleanup
    invoke-static {p4}, Lfixture/SharedTails;->cleanup(I)V
    return v1
  :outer_all
    move-exception v8
    invoke-static {p4}, Lfixture/SharedTails;->cleanup(I)V
    throw v8
  :switch_data
    .packed-switch 0
        :case_zero
        :case_one
        :case_two
        :tail_b
    .end packed-switch
.end method

.method public static conditional(IIII)I
    .locals 3
    move v0, p2
    move v1, p3
    packed-switch p0, :conditional_data
    const/16 v0, -9
    return v0
  :conditional_one
    add-int/lit8 v0, p2, 1
    add-int/lit8 v1, p3, 2
    goto :conditional_a
  :conditional_two
    add-int/lit8 v0, p2, 7
    add-int/lit8 v1, p3, 8
    goto :conditional_b
  :conditional_zero
    if-eqz p1, :conditional_b
  :conditional_a
    const/16 v2, 60
    invoke-static {v2}, Lfixture/SharedTails;->mark(I)V
    add-int/2addr v0, v1
  :conditional_b
    const/16 v2, 61
    invoke-static {v2}, Lfixture/SharedTails;->mark(I)V
    sub-int/2addr v0, v1
    return v0
  :conditional_data
    .packed-switch 0
        :conditional_zero
        :conditional_one
        :conditional_two
    .end packed-switch
.end method

.method public static foreign(II)I
    .locals 4
    const/4 v0, 0
    if-nez p1, :foreign_zero
    packed-switch p0, :foreign_data
    const/4 v0, -1
    return v0
  :foreign_zero
    const/16 v1, 10
    invoke-static {v1}, Lfixture/SharedTails;->mark(I)V
    add-int/lit8 v0, v0, 1
    goto :foreign_a
  :foreign_one
    const/16 v1, 20
    invoke-static {v1}, Lfixture/SharedTails;->mark(I)V
    add-int/lit8 v0, v0, 2
    goto :foreign_a
  :foreign_two
    const/16 v1, 30
    invoke-static {v1}, Lfixture/SharedTails;->mark(I)V
    add-int/lit8 v0, v0, 3
    goto :foreign_b
  :foreign_a
    add-int/lit8 v0, v0, 3
  :foreign_b
    add-int/lit8 v0, v0, 7
    return v0
  :foreign_data
    .packed-switch 0
        :foreign_zero
        :foreign_one
        :foreign_two
    .end packed-switch
.end method
