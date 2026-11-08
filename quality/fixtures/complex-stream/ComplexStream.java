package fixture;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.stream.Collectors;
import java.util.stream.IntStream;

/** Combined fixture: stream pipelines, method references, Optional. */
public final class ComplexStream {
    public List<Integer> evensDoubled(List<Integer> values) {
        return values.stream()
                .filter(value -> value % 2 == 0)
                .map(value -> value * 2)
                .collect(Collectors.toList());
    }

    public List<String> upper(List<String> values) {
        return values.stream().map(String::trim).map(String::toUpperCase).collect(Collectors.toList());
    }

    public int sumOver(int limit) {
        return IntStream.range(0, limit).filter(value -> value % 3 == 0).sum();
    }

    public int firstPositive(List<Integer> values) {
        Optional<Integer> found = values.stream().filter(value -> value > 0).findFirst();
        return found.orElse(-1);
    }

    public List<Integer> fromArrays(int[] first, int[] second) {
        List<Integer> out = new ArrayList<>();
        IntStream.concat(IntStream.of(first), IntStream.of(second)).forEach(out::add);
        return out;
    }
}
