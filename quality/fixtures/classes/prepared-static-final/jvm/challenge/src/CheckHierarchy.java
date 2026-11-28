public final class CheckHierarchy {
    private static void compiled() {
        HierarchyEffects.record("compiled:" + ReadStaticHierarchy.compiledLong() + ":"
                + (ReadStaticHierarchy.compiledText() == "hierarchy"));
    }

    private static void read(String label) {
        HierarchyEffects.record("read:" + label);
        try {
            String value = switch (label) {
                case "long" -> "long:" + ReadStaticHierarchy.inheritedLong();
                case "null" -> "null:" + (ReadStaticHierarchy.directNull() == null);
                case "alias" -> "alias:" + ReadStaticHierarchy.activeAlias();
                default -> throw new AssertionError(label);
            };
            HierarchyEffects.record("value:" + value);
        } catch (ExceptionInInitializerError error) {
            Throwable cause = error.getCause();
            if (cause == HierarchyEffects.ROOT_FAULT) {
                HierarchyEffects.record("initial:root");
            } else if (cause == HierarchyEffects.ALIAS_FAULT) {
                HierarchyEffects.record("initial:alias");
            } else {
                HierarchyEffects.record("initial:unexpected");
            }
        } catch (NoClassDefFoundError error) {
            HierarchyEffects.record("prior:" + label);
        }
    }

    public static void main(String[] args) {
        String scenario = args[0];
        HierarchyEffects.failRoot = scenario.startsWith("root-fault");
        HierarchyEffects.failAlias = scenario.equals("alias-fault");
        compiled();
        switch (scenario) {
            case "compile-only" -> { }
            case "inherited-first" -> {
                read("long");
                read("null");
                read("alias");
                compiled();
            }
            case "declaring-first" -> {
                read("null");
                read("long");
                read("alias");
                compiled();
            }
            case "alias-first" -> {
                read("alias");
                read("long");
                read("null");
                compiled();
            }
            case "root-fault-through-alias" -> {
                read("long");
                read("null");
                read("long");
                read("alias");
                compiled();
            }
            case "alias-fault" -> {
                read("alias");
                read("long");
                read("null");
                read("alias");
                compiled();
            }
            default -> throw new AssertionError(scenario);
        }
        System.out.println(HierarchyEffects.trace());
    }
}
