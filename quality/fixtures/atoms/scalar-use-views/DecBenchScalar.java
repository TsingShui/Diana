package fixture;

final class DecBenchScalar {
    private interface Action { Object invoke(); }

    private static void observe(String scenario, Action action) {
        try {
            Object result = action.invoke();
            if (result instanceof String text) {
                StringBuilder units = new StringBuilder();
                for (int unit : text.chars().toArray()) {
                    if (!units.isEmpty()) units.append(',');
                    units.append(unit);
                }
                System.out.println(scenario + "\tunits\t" + units);
            } else {
                System.out.println(scenario + "\tint\t" + result);
            }
        } catch (Throwable exception) {
            System.out.println(scenario + "\tthrow\t" + exception.getClass().getName());
        }
    }

    public static void main(String[] args) {
        observe("array-value", () -> ScalarViews.array(new char[]{'A'}));
        observe("array-empty", () -> ScalarViews.array(new char[0]));
        observe("array-null", () -> ScalarViews.array(null));
        observe("boundaries", ScalarViews::boundaries);
        observe("carrier-value", () -> ScalarViews.carrier("A"));
        observe("carrier-empty", () -> ScalarViews.carrier(""));
        observe("carrier-null", () -> ScalarViews.carrier(null));
        observe("escaped", () -> ScalarViews.escaped("'x"));
        observe("literal", () -> ScalarViews.literal("a/b/"));
        observe("parameter", () -> ScalarViews.parameter('A'));
    }
}
