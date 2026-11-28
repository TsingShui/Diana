import java.util.ArrayList;
import java.util.List;

public final class CrossedNumerics {
    public static void run(char unit, byte signedOctet) {
        CrossedNumerics receiver = new CrossedNumerics(unit, (int) unit);
        receiver.mix(unit, (int) unit);
        receiver.mix((int) unit, unit);
        receiver.octet(signedOctet, (int) signedOctet);
        receiver.octet((int) signedOctet, signedOctet);
        new CrossedNumerics((int) unit, unit);
    }

    public static void control(char unit, byte signedOctet, int word) {
        CrossedNumerics receiver = new CrossedNumerics(unit, unit);
        receiver.mix(unit, unit);
        receiver.octet(signedOctet, signedOctet);
        receiver.mix(word, word);
        receiver.octet(word, word);
    }

    public CrossedNumerics(char first, char second) {
        record("constructor:CC", first, second);
    }

    public CrossedNumerics(char first, int second) {
        record("constructor:CI", first, second);
    }

    public CrossedNumerics(int first, char second) {
        record("constructor:IC", first, second);
    }

    public CrossedNumerics(int first, int second) {
        record("constructor:II", first, second);
    }

    public void mix(char first, char second) {
        record("char:CC", first, second);
    }

    public void mix(char first, int second) {
        record("char:CI", first, second);
    }

    public void mix(int first, char second) {
        record("char:IC", first, second);
    }

    public void mix(int first, int second) {
        record("char:II", first, second);
    }

    public void octet(byte first, byte second) {
        record("byte:BB", first, second);
    }

    public void octet(byte first, int second) {
        record("byte:BI", first, second);
    }

    public void octet(int first, byte second) {
        record("byte:IB", first, second);
    }

    public void octet(int first, int second) {
        record("byte:II", first, second);
    }

    private static final List<String> events = new ArrayList<>();

    private static void record(String signature, int first, int second) {
        events.add(signature + ":" + first + ":" + second);
    }

    public static void main(String[] arguments) {
        char unit = (char) Integer.parseInt(arguments[1]);
        byte signedOctet = (byte) Integer.parseInt(arguments[2]);
        int word = Integer.parseInt(arguments[3]);
        if (arguments[0].equals("run")) {
            run(unit, signedOctet);
        } else {
            control(unit, signedOctet, word);
        }
        System.out.println(String.join(",", events));
    }
}
