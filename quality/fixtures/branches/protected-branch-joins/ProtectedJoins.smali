.class public final Lfixture/ProtectedJoins;
.super Ljava/lang/Object;

.method public static native before(I)I
.end method
.method public static native left(I)I
.end method
.method public static native right(I)I
.end method
.method public static native join(I)I
.end method
.method public static native recover(I)I
.end method
.method public static native after(I)I
.end method

.method public static route(II)I
    .locals 2
    invoke-static {p1}, Lfixture/ProtectedJoins;->before(I)I
    move-result v0
    :begin
    if-eqz p0, :left
    invoke-static {v0}, Lfixture/ProtectedJoins;->right(I)I
    move-result v1
    goto :join
    :left
    invoke-static {v0}, Lfixture/ProtectedJoins;->left(I)I
    move-result v1
    :join
    invoke-static {v1}, Lfixture/ProtectedJoins;->join(I)I
    move-result v0
    :end
    .catch Ljava/lang/IllegalStateException; {:begin .. :end} :caught
    invoke-static {v0}, Lfixture/ProtectedJoins;->after(I)I
    move-result v0
    return v0
    :caught
    move-exception v1
    invoke-static {v0}, Lfixture/ProtectedJoins;->recover(I)I
    move-result v0
    return v0
.end method

.method public static loopRoute(II)I
    .locals 2
    invoke-static {p1}, Lfixture/ProtectedJoins;->before(I)I
    move-result v0
    :head
    if-lez p0, :done
    :begin
    and-int/lit8 v1, p0, 0x1
    if-eqz v1, :left
    invoke-static {v0}, Lfixture/ProtectedJoins;->right(I)I
    move-result v0
    goto :check
    :left
    invoke-static {v0}, Lfixture/ProtectedJoins;->left(I)I
    move-result v0
    :check
    if-eqz v0, :next
    invoke-static {v0}, Lfixture/ProtectedJoins;->join(I)I
    move-result v0
    :end
    .catch Ljava/lang/IllegalStateException; {:begin .. :end} :caught
    :next
    add-int/lit8 p0, p0, -0x1
    goto :head
    :caught
    move-exception v1
    invoke-static {v0}, Lfixture/ProtectedJoins;->recover(I)I
    move-result v0
    return v0
    :done
    invoke-static {v0}, Lfixture/ProtectedJoins;->after(I)I
    move-result v0
    return v0
.end method

.method public static wideRoute(II)I
    .locals 2
    add-int v0, p0, p1
    invoke-static {v0}, Lfixture/ProtectedJoins;->before(I)I
    move-result v0
    :left_1
    invoke-static {v0}, Lfixture/ProtectedJoins;->left(I)I
    move-result v0
    :end_left_1
    .catch Ljava/lang/IllegalStateException; {:left_1 .. :end_left_1} :failed_1
    add-int/lit8 v0, v0, 0x1
    :right_1
    invoke-static {v0}, Lfixture/ProtectedJoins;->right(I)I
    move-result v0
    :end_right_1
    .catch Ljava/lang/IllegalStateException; {:right_1 .. :end_right_1} :failed_1
    invoke-static {v0}, Lfixture/ProtectedJoins;->after(I)I
    move-result v0
    :left_2
    invoke-static {v0}, Lfixture/ProtectedJoins;->left(I)I
    move-result v0
    :end_left_2
    .catch Ljava/lang/IllegalStateException; {:left_2 .. :end_left_2} :failed_2
    add-int/lit8 v0, v0, 0x2
    :right_2
    invoke-static {v0}, Lfixture/ProtectedJoins;->right(I)I
    move-result v0
    :end_right_2
    .catch Ljava/lang/IllegalStateException; {:right_2 .. :end_right_2} :failed_2
    invoke-static {v0}, Lfixture/ProtectedJoins;->after(I)I
    move-result v0
    :left_3
    invoke-static {v0}, Lfixture/ProtectedJoins;->left(I)I
    move-result v0
    :end_left_3
    .catch Ljava/lang/IllegalStateException; {:left_3 .. :end_left_3} :failed_3
    add-int/lit8 v0, v0, 0x3
    :right_3
    invoke-static {v0}, Lfixture/ProtectedJoins;->right(I)I
    move-result v0
    :end_right_3
    .catch Ljava/lang/IllegalStateException; {:right_3 .. :end_right_3} :failed_3
    invoke-static {v0}, Lfixture/ProtectedJoins;->after(I)I
    move-result v0
    :left_4
    invoke-static {v0}, Lfixture/ProtectedJoins;->left(I)I
    move-result v0
    :end_left_4
    .catch Ljava/lang/IllegalStateException; {:left_4 .. :end_left_4} :failed_4
    add-int/lit8 v0, v0, 0x4
    :right_4
    invoke-static {v0}, Lfixture/ProtectedJoins;->right(I)I
    move-result v0
    :end_right_4
    .catch Ljava/lang/IllegalStateException; {:right_4 .. :end_right_4} :failed_4
    invoke-static {v0}, Lfixture/ProtectedJoins;->after(I)I
    move-result v0
    :left_5
    invoke-static {v0}, Lfixture/ProtectedJoins;->left(I)I
    move-result v0
    :end_left_5
    .catch Ljava/lang/IllegalStateException; {:left_5 .. :end_left_5} :failed_5
    add-int/lit8 v0, v0, 0x5
    :right_5
    invoke-static {v0}, Lfixture/ProtectedJoins;->right(I)I
    move-result v0
    :end_right_5
    .catch Ljava/lang/IllegalStateException; {:right_5 .. :end_right_5} :failed_5
    invoke-static {v0}, Lfixture/ProtectedJoins;->after(I)I
    move-result v0
    :left_6
    invoke-static {v0}, Lfixture/ProtectedJoins;->left(I)I
    move-result v0
    :end_left_6
    .catch Ljava/lang/IllegalStateException; {:left_6 .. :end_left_6} :failed_6
    add-int/lit8 v0, v0, 0x6
    :right_6
    invoke-static {v0}, Lfixture/ProtectedJoins;->right(I)I
    move-result v0
    :end_right_6
    .catch Ljava/lang/IllegalStateException; {:right_6 .. :end_right_6} :failed_6
    invoke-static {v0}, Lfixture/ProtectedJoins;->after(I)I
    move-result v0
    return v0
    :failed_1
    move-exception v1
    add-int/lit8 v0, v0, -0x1
    invoke-static {v0}, Lfixture/ProtectedJoins;->recover(I)I
    move-result v0
    return v0
    :failed_2
    move-exception v1
    add-int/lit8 v0, v0, -0x2
    invoke-static {v0}, Lfixture/ProtectedJoins;->recover(I)I
    move-result v0
    return v0
    :failed_3
    move-exception v1
    add-int/lit8 v0, v0, -0x3
    invoke-static {v0}, Lfixture/ProtectedJoins;->recover(I)I
    move-result v0
    return v0
    :failed_4
    move-exception v1
    add-int/lit8 v0, v0, -0x4
    invoke-static {v0}, Lfixture/ProtectedJoins;->recover(I)I
    move-result v0
    return v0
    :failed_5
    move-exception v1
    add-int/lit8 v0, v0, -0x5
    invoke-static {v0}, Lfixture/ProtectedJoins;->recover(I)I
    move-result v0
    return v0
    :failed_6
    move-exception v1
    add-int/lit8 v0, v0, -0x6
    invoke-static {v0}, Lfixture/ProtectedJoins;->recover(I)I
    move-result v0
    return v0
.end method
