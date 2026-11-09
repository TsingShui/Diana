.class public final Lfixture/ProtectedExits;
.super Ljava/lang/Object;
.field public static trace:I

.method public static mark(I)V
    .locals 2
    sget v0, Lfixture/ProtectedExits;->trace:I
    const/16 v1, 31
    mul-int/2addr v0, v1
    add-int/2addr v0, p0
    sput v0, Lfixture/ProtectedExits;->trace:I
    return-void
.end method

.method public static probe(III)I
    .locals 2
    add-int/lit8 v0, p0, 100
    invoke-static {v0}, Lfixture/ProtectedExits;->mark(I)V
    const/4 v0, 1
    if-ne p2, v0, :probe_compare
    if-ne p0, v0, :probe_compare
    new-instance v0, Ljava/lang/NumberFormatException;
    const-string v1, "probe"
    invoke-direct {v0, v1}, Ljava/lang/NumberFormatException;-><init>(Ljava/lang/String;)V
    throw v0
  :probe_compare
    if-ne p0, p1, :probe_zero
    const/4 v0, 1
    return v0
  :probe_zero
    const/4 v0, 0
    return v0
.end method

.method public static read(II)I
    .locals 2
    add-int/lit16 v0, p0, 200
    invoke-static {v0}, Lfixture/ProtectedExits;->mark(I)V
    const/4 v0, 2
    if-eq p1, v0, :read_key
    const/4 v0, 5
    if-ne p1, v0, :read_value
  :read_key
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

.method public static cleanup(II)V
    .locals 2
    add-int/lit16 v0, p0, 300
    invoke-static {v0}, Lfixture/ProtectedExits;->mark(I)V
    const/4 v0, 3
    if-ne p1, v0, :cleanup_second_mode
    const/4 v0, 7
    if-eq p0, v0, :cleanup_fault
    goto :cleanup_done
  :cleanup_second_mode
    const/4 v0, 4
    if-eq p1, v0, :cleanup_second_code
    const/4 v0, 5
    if-ne p1, v0, :cleanup_done
  :cleanup_second_code
    const/16 v0, 8
    if-ne p0, v0, :cleanup_done
  :cleanup_fault
    new-instance v0, Ljava/lang/IllegalArgumentException;
    const-string v1, "cleanup"
    invoke-direct {v0, v1}, Ljava/lang/IllegalArgumentException;-><init>(Ljava/lang/String;)V
    throw v0
  :cleanup_done
    return-void
.end method

.method public static nested(IIII)I
    .locals 8
    const/4 v0, 0
    const/4 v1, 0
  :nested_start
  :nested_outer
    if-ge v0, p0, :nested_normal
    const/4 v2, 0
  :nested_inner
    if-ge v2, p1, :nested_outer_update
    mul-int/lit8 v3, v0, 10
    add-int/2addr v3, v2
    invoke-static {v3, p2, p3}, Lfixture/ProtectedExits;->probe(III)I
    move-result v4
    if-eqz v4, :nested_read
    new-instance v5, Ljava/lang/IllegalStateException;
    const-string v6, "guard"
    invoke-direct {v5, v6}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v5
  :nested_read
    invoke-static {v3, p3}, Lfixture/ProtectedExits;->read(II)I
    move-result v4
    add-int/2addr v1, v4
    add-int/lit8 v2, v2, 1
    goto :nested_inner
  :nested_outer_update
    add-int/lit8 v0, v0, 1
    goto :nested_outer
  :nested_end
    .catchall {:nested_start .. :nested_end} :nested_all
  :nested_normal
    const/4 v4, 7
    invoke-static {v4, p3}, Lfixture/ProtectedExits;->cleanup(II)V
    const/16 v4, 8
    invoke-static {v4, p3}, Lfixture/ProtectedExits;->cleanup(II)V
    return v1
  :nested_all
    move-exception v7
    const/4 v4, 7
    invoke-static {v4, p3}, Lfixture/ProtectedExits;->cleanup(II)V
    const/16 v4, 8
    invoke-static {v4, p3}, Lfixture/ProtectedExits;->cleanup(II)V
    throw v7
.end method

.method public static early(III)I
    .locals 5
    const/4 v0, 0
    const/4 v1, 0
  :early_start
  :early_head
    if-ge v0, p0, :early_normal
    if-ne v0, p1, :early_read
    return v1
  :early_read
    invoke-static {v0, p2}, Lfixture/ProtectedExits;->read(II)I
    move-result v2
    add-int/2addr v1, v2
    add-int/lit8 v0, v0, 1
    goto :early_head
  :early_end
    .catchall {:early_start .. :early_end} :early_all
  :early_normal
    const/4 v3, 7
    invoke-static {v3, p2}, Lfixture/ProtectedExits;->cleanup(II)V
    const/16 v3, 8
    invoke-static {v3, p2}, Lfixture/ProtectedExits;->cleanup(II)V
    return v1
  :early_all
    move-exception v4
    const/4 v3, 7
    invoke-static {v3, p2}, Lfixture/ProtectedExits;->cleanup(II)V
    const/16 v3, 8
    invoke-static {v3, p2}, Lfixture/ProtectedExits;->cleanup(II)V
    throw v4
.end method

.method public static switchExit(IIII)I
    .locals 7
    const/4 v0, 0
    const/4 v1, 0
  :switch_start
  :switch_head
    if-ge v0, p0, :switch_normal
    const/4 v2, 0
    packed-switch p1, :switch_data
  :switch_escape
    const/16 v5, 90
    invoke-static {v5}, Lfixture/ProtectedExits;->mark(I)V
    goto :switch_normal
  :switch_zero
    const/16 v5, 10
    invoke-static {v5}, Lfixture/ProtectedExits;->mark(I)V
    add-int/lit8 v1, v1, 1
    add-int/lit8 v2, v2, 1
    goto :switch_one
  :switch_one
    const/16 v5, 20
    invoke-static {v5}, Lfixture/ProtectedExits;->mark(I)V
    add-int/lit8 v1, v1, 2
    add-int/lit8 v2, v2, 1
    if-lt v2, p2, :switch_zero
    goto :switch_update
  :switch_two
    const/16 v5, 30
    invoke-static {v5}, Lfixture/ProtectedExits;->mark(I)V
    add-int/lit8 v1, v1, 3
  :switch_update
    add-int/lit8 v0, v0, 1
    goto :switch_head
  :switch_end
    .catchall {:switch_start .. :switch_end} :switch_all
  :switch_normal
    const/4 v5, 7
    invoke-static {v5, p3}, Lfixture/ProtectedExits;->cleanup(II)V
    const/16 v5, 8
    invoke-static {v5, p3}, Lfixture/ProtectedExits;->cleanup(II)V
    return v1
  :switch_all
    move-exception v6
    const/4 v5, 7
    invoke-static {v5, p3}, Lfixture/ProtectedExits;->cleanup(II)V
    const/16 v5, 8
    invoke-static {v5, p3}, Lfixture/ProtectedExits;->cleanup(II)V
    throw v6
  :switch_data
    .packed-switch 0
        :switch_zero
        :switch_one
        :switch_two
    .end packed-switch
.end method

.method public static recover(III)I
    .locals 7
    const/4 v0, 0
    const/4 v1, 0
  :recover_head
    if-ge v0, p0, :recover_done
  :recover_start
    invoke-static {v0, p1, p2}, Lfixture/ProtectedExits;->probe(III)I
    move-result v2
    if-eqz v2, :recover_read
    new-instance v3, Ljava/lang/IllegalStateException;
    const-string v4, "guard"
    invoke-direct {v3, v4}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v3
  :recover_read
    invoke-static {v0, p2}, Lfixture/ProtectedExits;->read(II)I
    move-result v2
    add-int/2addr v1, v2
  :recover_end
    .catchall {:recover_start .. :recover_end} :recover_all
    add-int/lit8 v0, v0, 1
    goto :recover_head
  :recover_all
    move-exception v6
    const/4 v5, 7
    invoke-static {v5, p2}, Lfixture/ProtectedExits;->cleanup(II)V
    const/16 v5, 8
    invoke-static {v5, p2}, Lfixture/ProtectedExits;->cleanup(II)V
    add-int/lit8 v0, v0, 2
    goto :recover_head
  :recover_done
    return v1
.end method

.method public static foreign(IIII)I
    .locals 8
    const/4 v0, 0
    const/4 v1, 0
    if-nez p3, :foreign_throw
  :foreign_start
  :foreign_head
    if-ge v0, p0, :foreign_normal
    invoke-static {v0, p1, p2}, Lfixture/ProtectedExits;->probe(III)I
    move-result v3
    if-eqz v3, :foreign_read
  :foreign_throw
    new-instance v5, Ljava/lang/IllegalStateException;
    const-string v6, "guard"
    invoke-direct {v5, v6}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v5
  :foreign_read
    invoke-static {v0, p2}, Lfixture/ProtectedExits;->read(II)I
    move-result v3
    add-int/2addr v1, v3
    add-int/lit8 v0, v0, 1
    goto :foreign_head
  :foreign_end
    .catchall {:foreign_start .. :foreign_end} :foreign_all
  :foreign_normal
    const/4 v4, 7
    invoke-static {v4, p2}, Lfixture/ProtectedExits;->cleanup(II)V
    const/16 v4, 8
    invoke-static {v4, p2}, Lfixture/ProtectedExits;->cleanup(II)V
    return v1
  :foreign_all
    move-exception v7
    const/4 v4, 7
    invoke-static {v4, p2}, Lfixture/ProtectedExits;->cleanup(II)V
    const/16 v4, 8
    invoke-static {v4, p2}, Lfixture/ProtectedExits;->cleanup(II)V
    throw v7
.end method
