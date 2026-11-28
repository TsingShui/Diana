package fixture;

public final class CheckInterface {
    public static void main(String[] args) {
        InterfaceEffects.add("before");
        String selected = args[0].equals("first") ? FaceConsumer.first() : FaceConsumer.second();
        InterfaceEffects.add("selected:" + selected);
        InterfaceEffects.add("first:" + RuntimeFace.readFirst());
        InterfaceEffects.add("second:" + RuntimeFace.readSecond());
        System.out.println(InterfaceEffects.trace());
    }
}
