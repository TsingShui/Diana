.class public final Lfixture/TypedCatchSharedCleanup;
.super Ljava/lang/Object;

.method public static native open(I)I
.end method

.method public static native read(I)I
.end method

.method public static native close(I)V
.end method

.method public static native parse(Ljava/lang/String;)I
.end method

# The protected inner body has one try item whose clauses are ordered
# NumberFormatException, NullPointerException, RuntimeException, then catch-all
# to the outer finally. Handler code needs the outer finally too, so the handler
# region is split around the nested detail body: the detail body has its own
# catch-all close handler, and the rethrow plus every other handler operation is
# covered by the outer catch-all. Shared close of the first resource is outside
# every protected range.
.method public static classify(Ljava/lang/String;I)I
    .locals 8
    const/4 v2, 0x0
    invoke-static {v2}, Lfixture/TypedCatchSharedCleanup;->open(I)I
    move-result v0
    const/4 v1, 0x0
    :inner_body_start
    const/4 v6, 0x1
    if-ne p1, v6, :not_nfe
    new-instance v3, Ljava/lang/NumberFormatException;
    const-string v6, "mode"
    invoke-direct {v3, v6}, Ljava/lang/NumberFormatException;-><init>(Ljava/lang/String;)V
    throw v3
    :not_nfe
    const/4 v6, 0x2
    if-ne p1, v6, :not_npe
    new-instance v3, Ljava/lang/NullPointerException;
    const-string v6, "mode"
    invoke-direct {v3, v6}, Ljava/lang/NullPointerException;-><init>(Ljava/lang/String;)V
    throw v3
    :not_npe
    const/4 v6, 0x3
    if-ne p1, v6, :not_ise
    new-instance v3, Ljava/lang/IllegalStateException;
    const-string v6, "mode"
    invoke-direct {v3, v6}, Ljava/lang/IllegalStateException;-><init>(Ljava/lang/String;)V
    throw v3
    :not_ise
    invoke-static {p0}, Lfixture/TypedCatchSharedCleanup;->parse(Ljava/lang/String;)I
    move-result v2
    move v1, v2
    :inner_body_end
    goto :shared_cleanup
    :multi_handler_body
    :multi_handler
    move-exception v3
    const/4 v6, 0x1
    invoke-static {v6}, Lfixture/TypedCatchSharedCleanup;->open(I)I
    move-result v4
    :detail_body
    invoke-static {v4}, Lfixture/TypedCatchSharedCleanup;->read(I)I
    move-result v5
    add-int/lit8 v5, v5, 0x64
    move v1, v5
    :detail_body_end
    invoke-static {v4}, Lfixture/TypedCatchSharedCleanup;->close(I)V
    goto :shared_cleanup
    :detail_handler
    move-exception v6
    invoke-static {v4}, Lfixture/TypedCatchSharedCleanup;->close(I)V
    throw v6
    :broad_handler
    move-exception v3
    const/4 v1, -0x2
    goto :shared_cleanup
    :shared_cleanup
    invoke-static {v0}, Lfixture/TypedCatchSharedCleanup;->close(I)V
    return v1
    :outer_handler
    move-exception v3
    invoke-static {v0}, Lfixture/TypedCatchSharedCleanup;->close(I)V
    throw v3
    .catch Ljava/lang/NumberFormatException; {:inner_body_start .. :inner_body_end} :multi_handler
    .catch Ljava/lang/NullPointerException; {:inner_body_start .. :inner_body_end} :multi_handler
    .catch Ljava/lang/RuntimeException; {:inner_body_start .. :inner_body_end} :broad_handler
    .catchall {:inner_body_start .. :inner_body_end} :outer_handler
    .catchall {:multi_handler_body .. :detail_body} :outer_handler
    .catchall {:detail_body .. :detail_body_end} :detail_handler
    .catchall {:detail_body_end .. :shared_cleanup} :outer_handler
.end method
