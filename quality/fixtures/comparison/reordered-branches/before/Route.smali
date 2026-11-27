.class public Llayout/Route;
.super Ljava/lang/Object;

.method public static stable(II)I
    .locals 1
    if-lez p0, :first
    goto :dispatch

    :dispatch
    if-eqz p1, :first
    goto :second

    :first
    add-int/lit8 v0, p1, -3
    goto :done

    :second
    mul-int/lit8 v0, p1, 3
    goto :done

    :done
    return v0
.end method

.method public static redirect(II)I
    .locals 1
    if-lez p0, :first
    goto :dispatch

    :dispatch
    if-eqz p1, :first
    goto :second

    :first
    xor-int/lit8 v0, p1, 13
    goto :done

    :second
    and-int/lit8 v0, p1, 7
    goto :done

    :done
    return v0
.end method
