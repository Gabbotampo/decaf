package demo;

import it.unimi.dsi.fastutil.objects.Object2BooleanAVLTreeMap;
import it.unimi.dsi.fastutil.objects.Object2BooleanLinkedOpenHashMap;
import it.unimi.dsi.fastutil.objects.Object2BooleanMap;
import it.unimi.dsi.fastutil.objects.Object2BooleanRBTreeMap;
import it.unimi.dsi.fastutil.objects.Object2BooleanSortedMap;
import it.unimi.dsi.fastutil.objects.ObjectBidirectionalIterator;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.Map;

/**
 * Oracle for fastutil 8.5.18 Object2Boolean*Map entry-set iteration with String keys.
 *
 * rows() is every case the runtime answers (`FastSortedEntrySet.iterator()` of the linked hash
 * map and the iterator it returns); countedRows() is the cases it leaves counted. Each prints one
 * line per case: <name><TAB><result>, an exception as "<exception-class-name>: <message>".
 *
 * Build/run (JDK 25, fastutil-8.5.18.jar on the class path):
 *   javac --release 21 -cp $FASTUTIL -d out demo/Probe.java
 *   java -cp $FASTUTIL:out demo.Probe          # rows()
 *   java -cp $FASTUTIL:out demo.Probe counted  # countedRows()
 */
public class Probe {

    interface Throwing<T> {
        T get() throws Throwable;
    }

    /** Run an action; on success return its string, on failure "<Class>: <msg>". */
    static String of(Throwing<String> f) {
        try {
            return f.get();
        } catch (Throwable t) {
            String msg = t.getMessage();
            return t.getClass().getName() + ": " + (msg == null ? "" : msg);
        }
    }

    /** The keys of each entry visited by an iterator, comma separated. */
    static String keys(Iterator<?> it) {
        StringBuilder sb = new StringBuilder();
        boolean first = true;
        while (it.hasNext()) {
            Object o = it.next();
            if (!first) {
                sb.append(',');
            }
            first = false;
            sb.append(String.valueOf(((Map.Entry<?, ?>) o).getKey()));
        }
        return sb.toString();
    }

    /** The keys of a list of entries, comma separated. */
    static String keys(List<Object2BooleanMap.Entry<String>> entries) {
        StringBuilder sb = new StringBuilder();
        for (Object2BooleanMap.Entry<String> e : entries) {
            if (sb.length() > 0) {
                sb.append(',');
            }
            sb.append(e.getKey());
        }
        return sb.toString();
    }

    static String keys(Object2BooleanLinkedOpenHashMap<String> map) {
        return keys(map.object2BooleanEntrySet().iterator());
    }

    static Object2BooleanLinkedOpenHashMap<String> linked() {
        Object2BooleanLinkedOpenHashMap<String> m = new Object2BooleanLinkedOpenHashMap<>();
        m.put("banana", true);
        m.put("apple", false);
        m.put("cherry", true);
        return m;
    }

    static StringBuilder out;

    static void line(String name, String result) {
        out.append(name).append('\t').append(result).append('\n');
    }

    public static void main(String[] args) {
        System.out.print(args.length > 0 ? countedRows() : rows());
    }

    public static String rows() {
        out = new StringBuilder();
        Object2BooleanLinkedOpenHashMap<String> linked = linked();

        // FastSortedEntrySet.iterator(): insertion order, read through java.util.Iterator.
        line("LINKED.iterator-order", of(() -> keys(linked.object2BooleanEntrySet().iterator())));
        // The same iterator, typed as the fastutil iterator (hasNext/next/getKey/getBooleanValue).
        line("LINKED.iterator-typed", of(() -> {
            ObjectBidirectionalIterator<Object2BooleanMap.Entry<String>> it =
                    linked.object2BooleanEntrySet().iterator();
            StringBuilder sb = new StringBuilder();
            while (it.hasNext()) {
                Object2BooleanMap.Entry<String> e = it.next();
                sb.append(e.getKey()).append('=').append(e.getBooleanValue()).append(';');
            }
            return sb.toString();
        }));
        // Every iterator() call is a new iterator with its own cursor.
        line("LINKED.iterator-independent", of(() -> {
            ObjectBidirectionalIterator<Object2BooleanMap.Entry<String>> a =
                    linked.object2BooleanEntrySet().iterator();
            ObjectBidirectionalIterator<Object2BooleanMap.Entry<String>> b =
                    linked.object2BooleanEntrySet().iterator();
            String first = a.next().getKey() + "," + a.next().getKey();
            return first + "|" + b.next().getKey() + "|" + (a != b);
        }));
        // Each next() is a new entry; the entries outlive the iteration.
        line("LINKED.iterator-retained", of(() -> {
            List<Object2BooleanMap.Entry<String>> kept = new ArrayList<>();
            Iterator<Object2BooleanMap.Entry<String>> it = linked.object2BooleanEntrySet().iterator();
            while (it.hasNext()) {
                kept.add(it.next());
            }
            return keys(kept) + "|" + kept.get(0).getBooleanValue() + kept.get(1).getBooleanValue()
                    + kept.get(2).getBooleanValue();
        }));
        // Past the end, and remove() with no entry to remove.
        line("LINKED.iterator-past-end", of(() -> {
            ObjectBidirectionalIterator<Object2BooleanMap.Entry<String>> it =
                    linked.object2BooleanEntrySet().iterator();
            it.next();
            it.next();
            it.next();
            boolean more = it.hasNext();
            it.next();
            return "no exception " + more;
        }));
        line("LINKED.iterator-remove-before-next", of(() -> {
            linked.object2BooleanEntrySet().iterator().remove();
            return "no exception";
        }));
        line("LINKED.iterator-remove-twice", of(() -> {
            Object2BooleanLinkedOpenHashMap<String> m = linked();
            ObjectBidirectionalIterator<Object2BooleanMap.Entry<String>> it =
                    m.object2BooleanEntrySet().iterator();
            it.next();
            it.remove();
            it.remove();
            return "no exception";
        }));
        // remove() through the iterator: the map shrinks and the iteration goes on. Nothing is
        // read from an entry that was returned before the removal.
        line("LINKED.iterator-remove-middle", of(() -> {
            Object2BooleanLinkedOpenHashMap<String> m = linked();
            ObjectBidirectionalIterator<Object2BooleanMap.Entry<String>> it =
                    m.object2BooleanEntrySet().iterator();
            int n = 0;
            while (it.hasNext()) {
                it.next();
                n++;
                if (n == 2) {
                    it.remove();
                }
            }
            return n + "|" + m.size() + "|" + keys(m) + "|" + m.containsKey("apple");
        }));
        line("LINKED.iterator-remove-last", of(() -> {
            Object2BooleanLinkedOpenHashMap<String> m = linked();
            Iterator<Object2BooleanMap.Entry<String>> it = m.object2BooleanEntrySet().iterator();
            StringBuilder seen = new StringBuilder();
            while (it.hasNext()) {
                Object2BooleanMap.Entry<String> e = it.next();
                seen.append(e.getKey()).append(e.getBooleanValue()).append(';');
                if (!it.hasNext()) {
                    it.remove();
                }
            }
            return seen + "|" + m.size() + "|" + keys(m) + "|" + m.containsKey("cherry");
        }));
        line("LINKED.iterator-remove-all", of(() -> {
            Object2BooleanLinkedOpenHashMap<String> m = linked();
            Iterator<Object2BooleanMap.Entry<String>> it = m.object2BooleanEntrySet().iterator();
            while (it.hasNext()) {
                it.next();
                it.remove();
            }
            return m.size() + "|" + m.isEmpty() + "|" + keys(m);
        }));
        // A key removed and put again moves to the end of the order.
        line("LINKED.iterator-reput", of(() -> {
            Object2BooleanLinkedOpenHashMap<String> m = linked();
            m.removeBoolean("banana");
            m.put("banana", false);
            m.put("apple", true);
            return keys(m);
        }));
        line("LINKED.iterator-empty", of(() -> {
            Object2BooleanLinkedOpenHashMap<String> m = new Object2BooleanLinkedOpenHashMap<>();
            ObjectBidirectionalIterator<Object2BooleanMap.Entry<String>> it =
                    m.object2BooleanEntrySet().iterator();
            boolean more = it.hasNext();
            it.next();
            return "no exception " + more;
        }));
        return out.toString();
    }

    public static String countedRows() {
        out = new StringBuilder();
        Object2BooleanLinkedOpenHashMap<String> linked = linked();

        // fastIterator() reuses one entry object for every next().
        line("LINKED.fastIterator-order", of(() -> keys(linked.object2BooleanEntrySet().fastIterator())));
        line("LINKED.fastIterator-retained", of(() -> {
            List<Object2BooleanMap.Entry<String>> kept = new ArrayList<>();
            Iterator<Object2BooleanMap.Entry<String>> it = linked.object2BooleanEntrySet().fastIterator();
            while (it.hasNext()) {
                kept.add(it.next());
            }
            return keys(kept);
        }));
        // fastutil's iterator reads the map live and never fails fast: a key put under it is
        // visited when it lands after the cursor.
        line("LINKED.iterator-put-during", of(() -> {
            Object2BooleanLinkedOpenHashMap<String> m = linked();
            Iterator<Object2BooleanMap.Entry<String>> it = m.object2BooleanEntrySet().iterator();
            StringBuilder seen = new StringBuilder();
            while (it.hasNext()) {
                String key = it.next().getKey();
                seen.append(key).append(';');
                if (key.equals("banana")) {
                    m.put("date", true);
                }
            }
            return seen.toString();
        }));
        line("LINKED.iterator-remove-during", of(() -> {
            Object2BooleanLinkedOpenHashMap<String> m = linked();
            Iterator<Object2BooleanMap.Entry<String>> it = m.object2BooleanEntrySet().iterator();
            StringBuilder seen = new StringBuilder();
            while (it.hasNext()) {
                String key = it.next().getKey();
                seen.append(key).append(';');
                if (key.equals("banana")) {
                    m.removeBoolean("cherry");
                }
            }
            return seen.toString();
        }));
        // An entry returned before a removal reads the map as it is after it.
        line("LINKED.iterator-read-after-remove", of(() -> {
            Object2BooleanLinkedOpenHashMap<String> m = linked();
            Iterator<Object2BooleanMap.Entry<String>> it = m.object2BooleanEntrySet().iterator();
            StringBuilder seen = new StringBuilder();
            while (it.hasNext()) {
                Object2BooleanMap.Entry<String> e = it.next();
                seen.append(e.getKey()).append(';');
                if (e.getKey().equals("apple")) {
                    it.remove();
                }
            }
            return seen.toString();
        }));
        // The tree maps' entry set is an ObjectSortedSet in natural key order, not a FastSortedEntrySet.
        Object2BooleanAVLTreeMap<String> avl = new Object2BooleanAVLTreeMap<>();
        avl.put("banana", true);
        avl.put("apple", false);
        avl.put("cherry", true);
        Object2BooleanRBTreeMap<String> rbt = new Object2BooleanRBTreeMap<>();
        rbt.put("banana", true);
        rbt.put("apple", false);
        rbt.put("cherry", true);
        line("AVL.iterator-order", of(() -> keys(avl.object2BooleanEntrySet().iterator())));
        line("RBT.iterator-order", of(() -> keys(rbt.object2BooleanEntrySet().iterator())));
        Object2BooleanSortedMap<String> sorted = avl;
        line("AVL.sorted-iterator-order", of(() -> keys(sorted.object2BooleanEntrySet().iterator())));
        return out.toString();
    }
}
