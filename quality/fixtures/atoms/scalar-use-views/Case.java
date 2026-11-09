package fixture;

/** Independently authored explanation; Case.smali is the assembly input. */
public final class ScalarViews {
    static int probe(char value) { return 1000 + value; }
    static int probe(int value) { return 2000 + value; }
    static int probe(byte value) { return 3000 + value; }
    static int probe(short value) { return 4000 + value; }
    static int probe(boolean value) { return value ? 5001 : 5000; }

    static int carrier(String text) {
        char unit = text.charAt(0);
        int character = probe(unit);
        int word = probe((int) unit);
        return character + word;
    }

    static String array(char[] units) {
        StringBuilder output = new StringBuilder();
        char unit = units[0];
        output.append(unit);
        output.append((int) unit);
        return output.toString();
    }

    static String literal(String text) { return text.replace('/', '-'); }
    static String escaped(String text) { return text.replace('\'', '\ud800'); }

    static int boundaries() {
        int result = probe('\uffff');
        result += probe((byte) -128);
        result += probe((short) -32768);
        result += probe(true);
        result += probe(false);
        return result;
    }

    static int parameter(char unit) { return probe(unit); }
}
