package fixture
object SuspendChecksum {
    suspend fun total(limit: Int, bias: Int): Int {
        var sum=0
        var index=0
        var checksum=bias
        while(index<limit) {
            val first=load(index)
            record(checksum)
            if(first<0) break
            val second=load(first+1)
            sum+=first+second
            checksum+=index
            record(sum)
            index++
        }
        return sum+checksum
    }
    private external suspend fun load(value:Int):Int
    private external fun record(value:Int)
}
