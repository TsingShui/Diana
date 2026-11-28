package fixture;

public final class CheckPreparedDefaults {
    private static void read(String field) {
        PreparationEvents.trace.add("read:" + field);
        try {
            Object value = field.equals("zero") ? ReadPreparedDefaults.preparedZero()
                    : ReadPreparedDefaults.preparedNull();
            PreparationEvents.trace.add("value:" + value);
        } catch (ExceptionInInitializerError error) {
            PreparationEvents.trace.add("initializer:" + (error.getCause() == PreparationEvents.fault));
        } catch (NoClassDefFoundError error) {
            PreparationEvents.trace.add("erroneous");
        }
    }

    public static void main(String[] args) {
        String scenario = args[0];
        PreparationEvents.fail = scenario.startsWith("fault");
        PreparationEvents.trace.add("encoded:" + ReadPreparedDefaults.encodedZero());
        if (!scenario.equals("control")) {
            boolean zeroFirst = scenario.endsWith("zero");
            read(zeroFirst ? "zero" : "null");
            read(zeroFirst ? "null" : "zero");
            PreparationEvents.trace.add("encoded:" + ReadPreparedDefaults.encodedZero());
        }
        System.out.println(String.join(",", PreparationEvents.trace));
    }
}
