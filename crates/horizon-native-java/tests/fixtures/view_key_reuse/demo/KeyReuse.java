package demo;

import java.util.*;

/**
 * Views numbered by their table's size reused a live view's key once a collection shrank the table
 * (#243's review): a view kept in a static then read another view's contents. Unmodifiable views
 * and reversed views are kept here while many others die across callbacks.
 */
public final class KeyReuse {
    static Set<String> keptSet;
    static List<String> keptReversed;
    static List<String> keptList;
    static Map<String, Integer> keptMap;

    public static String setup() {
        for (int i = 0; i < 5; i++) {
            Collections.unmodifiableSet(new HashSet<>(Set.of("g" + i)));
            new ArrayList<>(List.of("r" + i)).reversed();
        }
        Set<String> set = new HashSet<>();
        set.add("kept");
        keptSet = Collections.unmodifiableSet(set);
        keptReversed = new ArrayList<>(List.of("a", "b", "c")).reversed();
        keptList = Collections.unmodifiableList(new ArrayList<>(List.of("x", "y")));
        Map<String, Integer> map = new HashMap<>();
        map.put("m", 1);
        keptMap = Collections.unmodifiableMap(map);
        return "ok";
    }

    /** Views that die at once. */
    public static String garbage() {
        for (int i = 0; i < 20; i++) {
            Collections.unmodifiableSet(new HashSet<>(Set.of("t" + i)));
            Collections.unmodifiableList(new ArrayList<>(List.of("w" + i)));
            Collections.unmodifiableMap(new HashMap<>(Map.of("k" + i, i)));
            new ArrayList<>(List.of("q" + i, "z")).reversed();
        }
        return "ok";
    }

    public static String check() {
        return keptSet + "|" + keptReversed + "|" + keptList + "|" + keptMap + "|" + keptSet.size()
            + keptReversed.size() + keptList.size() + keptMap.size();
    }

    public static void main(String[] args) {
        System.out.println("setup=" + setup());
        for (int i = 0; i < 3; i++) {
            garbage();
        }
        System.out.println("check=" + check());
    }
}
