public final class CheckGenericArray {
    public static void main(String[] args) {
        int mode = Integer.parseInt(args[1]);
        GenericArrayEffects.failLeading = mode == 4 || mode == 5;
        if (mode == 5) mode = 2;
        String result;
        try {
            result = switch (args[0]) {
                case "object" -> GenericArrayOperand.object(mode);
                case "array" -> GenericArrayOperand.array(mode);
                default -> throw new AssertionError(args[0]);
            };
        } catch (ClassCastException failure) {
            result = "cast-fault";
        } catch (IllegalStateException failure) {
            if (failure == GenericArrayEffects.LEADING) {
                result = "leading-fault";
            } else if (failure == GenericArrayEffects.NESTED) {
                result = "nested-fault";
            } else if (failure == GenericArrayEffects.CAPTURE) {
                result = "capture-fault";
            } else {
                throw failure;
            }
        }
        System.out.printf("%s/%d%n", result, GenericArrayEffects.trace);
    }
}
