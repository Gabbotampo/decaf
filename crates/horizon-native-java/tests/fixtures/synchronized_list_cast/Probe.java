package demo;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

public final class Probe {
    public static String probe() {
        Object value = Collections.synchronizedList(new ArrayList<>(List.of("x")));
        List<?> cast = (List<?>) value;
        return Integer.toString(cast.size());
    }

    public static void main(String[] args) {
        System.out.println(probe());
    }
}
