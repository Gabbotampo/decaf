package demo;

import java.util.*;
import kotlin.Lazy;
import kotlin.LazyKt;
import kotlin.enums.EnumEntries;
import kotlin.enums.EnumEntriesKt;
import kotlin.jvm.functions.Function0;
import kotlin.jvm.internal.Intrinsics;
import kotlin.collections.CollectionsKt;
import kotlin.collections.MapsKt;
import kotlin.collections.SetsKt;

public final class Probe {
    enum E { A, B }

    private static String exception(int which) {
        try {
            switch (which) {
                case 0: Intrinsics.checkNotNull(null); break;
                case 1: Intrinsics.checkNotNull(null, "hello"); break;
                case 2: Intrinsics.checkNotNullParameter(null, "arg"); break;
                case 3: Intrinsics.checkParameterIsNotNull(null, "arg"); break;
                case 4: Intrinsics.checkNotNullExpressionValue(null, "expr"); break;
                case 5: Intrinsics.checkExpressionValueIsNotNull(null, "expr"); break;
                case 6: Intrinsics.throwUninitializedPropertyAccessException("foo"); break;
                default: return "bad";
            }
            return "ok";
        } catch (Throwable t) {
            return t.getClass().getName() + "|" + t.getMessage();
        }
    }

    public static String enumEntries() {
        EnumEntries<E> a = EnumEntriesKt.enumEntries(E.values());
        EnumEntries<E> b = EnumEntriesKt.enumEntries(E.values());
        return a.getClass().getName() + "|" + a.size() + "|" + a.get(0).name() + a.get(1).name()
            + "|" + (a == b) + "|" + a.equals(b) + "|" + exceptionAdd(a) + "|" + a.toString();
    }

    private static String exceptionAdd(EnumEntries<E> a) {
        try { a.add(E.A); return "ok"; }
        catch (Throwable t) { return t.getClass().getName() + "|" + t.getMessage(); }
    }

    public static String intrinsics() {
        Intrinsics.checkNotNull("ok");
        Intrinsics.checkNotNull("ok", "message");
        Intrinsics.checkNotNullParameter("ok", "arg");
        Intrinsics.checkParameterIsNotNull("ok", "arg");
        Intrinsics.checkNotNullExpressionValue("ok", "expr");
        Intrinsics.checkExpressionValueIsNotNull("ok", "expr");
        return Intrinsics.areEqual((Object)null, (Object)null) + "|"
            + Intrinsics.areEqual((Object)null, (Object)"x") + "|"
            + Intrinsics.areEqual((Object)"x", (Object)"x") + "|"
            + exception(0) + "|" + exception(1) + "|" + exception(2) + "|" + exception(3)
            + "|" + exception(4) + "|" + exception(5) + "|" + exception(6);
    }

    public static String lazy() {
        final int[] calls = { 0 };
        Lazy<String> value = LazyKt.lazy(new Function0<String>() {
            public String invoke() { calls[0]++; return "v"; }
        });
        String before = value.toString();
        boolean initialized = value.isInitialized();
        String first = value.getValue();
        String second = value.getValue();
        return before + "|" + initialized + "|" + first + "|" + second + "|" + calls[0]
            + "|" + value.isInitialized() + "|" + value.toString();
    }

    public static String collections() {
        List<Object> list = CollectionsKt.emptyList();
        List<Object> list2 = CollectionsKt.listOf();
        List<Object> mutable = CollectionsKt.mutableListOf();
        Set<Object> set = SetsKt.emptySet();
        Map<Object,Object> map = MapsKt.emptyMap();
        return list.getClass().getName() + "|" + (list == list2) + "|" + (list == CollectionsKt.emptyList())
            + "|" + mutable.getClass().getName() + "|" + set.getClass().getName() + "|" + map.getClass().getName()
            + "|" + exceptionAddList(list);
    }

    private static String exceptionAddList(List<Object> list) {
        try { list.add("x"); return "ok"; }
        catch (Throwable t) { return t.getClass().getName() + "|" + t.getMessage(); }
    }
}
