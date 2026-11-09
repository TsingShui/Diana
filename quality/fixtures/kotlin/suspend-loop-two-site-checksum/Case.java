package fixture;
final class SuspendChecksum {
    static int total(int limit, int bias) {
        int sum=0, index=0, checksum=bias;
        while(index<limit) {
            int first=load(index);
            record(checksum);
            if(first<0) break;
            int second=load(first+1);
            sum+=first+second;
            checksum+=index;
            record(sum);
            index++;
        }
        return sum+checksum;
    }
    static native int load(int value);
    static native void record(int value);
}
