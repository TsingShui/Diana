public final class CheckNull {
    public static void main(String[] args) {
        String result = switch (args[0]) {
            case "object" -> NullableOverload.object(new NullableOverload<Number>());
            case "integer" -> NullableOverload.integer(new NullableOverload<Number>());
            case "guarded" ->
                NullableOverload.guarded(new NullableOverload<Number[]>(), Boolean.parseBoolean(args[1]));
            case "bounded" -> NullableOverload.bounded();
            default -> throw new AssertionError(args[0]);
        };
        System.out.printf("%s/%d%n", result, NullEffects.trace);
    }
}
