package fixture;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/** Classic algorithm: KMP search, word frequency and string reversal. */
public final class AlgoString {
    public static int[] prefix(String pattern) {
        int[] table = new int[pattern.length()];
        int length = 0;
        for (int index = 1; index < pattern.length(); index++) {
            while (length > 0 && pattern.charAt(index) != pattern.charAt(length)) {
                length = table[length - 1];
            }
            if (pattern.charAt(index) == pattern.charAt(length)) {
                length++;
            }
            table[index] = length;
        }
        return table;
    }

    public static int indexOf(String text, String pattern) {
        if (pattern.isEmpty()) {
            return 0;
        }
        int[] table = prefix(pattern);
        int matched = 0;
        for (int index = 0; index < text.length(); index++) {
            while (matched > 0 && text.charAt(index) != pattern.charAt(matched)) {
                matched = table[matched - 1];
            }
            if (text.charAt(index) == pattern.charAt(matched)) {
                matched++;
            }
            if (matched == pattern.length()) {
                return index - matched + 1;
            }
        }
        return -1;
    }

    public static List<String> topWords(String text) {
        Map<String, Integer> counts = new HashMap<>();
        for (String word : text.toLowerCase().split("\\s+")) {
            if (word.isEmpty()) {
                continue;
            }
            Integer seen = counts.get(word);
            counts.put(word, seen == null ? 1 : seen + 1);
        }
        List<String> words = new ArrayList<>();
        for (Map.Entry<String, Integer> entry : counts.entrySet()) {
            if (entry.getValue() > 1) {
                words.add(entry.getKey());
            }
        }
        return words;
    }

    public static String reverse(String text) {
        StringBuilder builder = new StringBuilder(text.length());
        for (int index = text.length() - 1; index >= 0; index--) {
            builder.append(text.charAt(index));
        }
        return builder.toString();
    }
}
