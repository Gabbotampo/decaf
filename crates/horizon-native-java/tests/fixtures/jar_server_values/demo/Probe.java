package demo;

import java.util.*;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.fabricmc.fabric.api.event.player.PlayerBlockBreakEvents;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.level.storage.LevelResource;

/**
 * A mod's own positions and resources against the game's (#245): the game hands out `1,2,3` and
 * `1,2,4` as broken blocks, and on a server tick `ask` meets them with its own `new BlockPos`,
 * `MutableBlockPos` and `LevelResource`. `Oracle.java` calls the same `ask` on the real 26.3
 * classes, with vanilla's own positions for the game's.
 */
public final class Probe {
    public static String log = "";
    public static String counted = "";
    static final List<BlockPos> broken = new ArrayList<>();

    static void p(String key, Object value) {
        log = log + key + "=" + value + "\n";
    }

    /** A subclass whose `getX` `Vec3i.equals` calls: Java's answer is its own. */
    static final class Odd extends BlockPos {
        Odd() { super(1, 2, 3); }
        @Override public int getX() { return 9; }
    }

    /** A subclass that moves itself through `Vec3i`'s protected setter. */
    static final class Mover extends BlockPos {
        Mover() { super(1, 2, 3); }
        void bump() { setZ(getZ() + 1); }
    }

    public void onInitialize() {
        PlayerBlockBreakEvents.AFTER.register((level, player, pos, state, entity) -> broken.add(pos));
        ServerTickEvents.END_SERVER_TICK.register(server -> ask(broken.get(0), broken.get(1)));
    }

    public static void ask(BlockPos h123, BlockPos h124) {
        BlockPos j123 = new BlockPos(1, 2, 3), j123b = new BlockPos(1, 2, 3);
        BlockPos.MutableBlockPos m = new BlockPos.MutableBlockPos(1, 2, 3);
        p("jar_text", j123 + "," + m);
        p("jar_hash", j123.hashCode() + "," + m.hashCode());
        p("jar_xyz", j123.getX() + "," + j123.getY() + "," + j123.getZ());
        p("eq", j123.equals(h123) + "," + h123.equals(j123) + "," + m.equals(h123) + "," + h123.equals(m) + ","
            + j123.equals(h124) + "," + h124.equals(j123) + "," + j123.equals(j123b) + "," + (j123 == j123b) + ","
            + j123.equals("x") + "," + j123.equals(null));
        Map<BlockPos, String> byHost = new HashMap<>();
        byHost.put(h123, "h");
        Map<BlockPos, String> byJar = new HashMap<>();
        byJar.put(j123, "j");
        p("map", byHost.get(j123) + "," + byHost.get(m) + "," + byJar.get(h123) + "," + byJar.get(h124) + ","
            + byHost.containsKey(new BlockPos(1, 2, 3)));
        Set<BlockPos> set = new HashSet<>(List.of(h123, h124));
        p("set", set.contains(j123) + "," + set.contains(new BlockPos(9, 9, 9)) + ","
            + new HashSet<BlockPos>(Arrays.asList(h123, j123, m, j123b)).size());
        p("compare", j123.compareTo(h124) + "," + h124.compareTo(j123) + "," + m.compareTo(h123));
        // The order a mixed `TreeSet` keeps, each element printed by itself.
        StringBuilder order = new StringBuilder();
        for (BlockPos pos : new TreeSet<BlockPos>(List.of(new BlockPos(3, 1, 0), h124, new BlockPos(-1, -2, -3),
                h123, new BlockPos(0, 1, 5)))) {
            order.append(pos).append(';');
        }
        p("tree", order);
        // A `MutableBlockPos` changed after it went into a `HashMap`: the map keeps the hash it
        // was put with.
        BlockPos.MutableBlockPos k = new BlockPos.MutableBlockPos(1, 2, 3);
        Map<BlockPos, String> moved = new HashMap<>();
        moved.put(k, "m");
        k.set(5, 5, 5);
        p("mutated", moved.get(h123) + "," + moved.get(new BlockPos(5, 5, 5)) + "," + moved.get(k) + ","
            + moved.size() + "," + moved.containsValue("m") + "," + moved.keySet().iterator().next());
        k.setY(7);
        p("mut_set", k + "," + new BlockPos.MutableBlockPos(1.5, -0.5, 2.0));
        LevelResource r = new LevelResource("data");
        Map<LevelResource, String> res = new HashMap<>();
        res.put(LevelResource.DATA, "d");
        p("res", r + "," + r.hashCode() + "," + r.equals(LevelResource.DATA) + "," + LevelResource.DATA.equals(r)
            + "," + res.get(r) + "," + r.id() + "," + r.equals(new LevelResource("root")));
        // Changes the game's own bodies make: each goes through `set(int, int, int)`.
        BlockPos.MutableBlockPos mv = new BlockPos.MutableBlockPos(1, 2, 3);
        mv.move(0, 0, 1);
        p("move", mv.equals(h124) + "," + mv.equals(h123) + "," + mv + "," + mv.hashCode() + "," + h124.equals(mv));
        BlockPos.MutableBlockPos md = new BlockPos.MutableBlockPos(1, 2, 3);
        md.move(Direction.SOUTH);
        String south = md.equals(h124) + "," + md;
        md.move(Direction.UP, 3);
        p("move_dir", south + "," + md);
        BlockPos.MutableBlockPos packed = new BlockPos.MutableBlockPos();
        packed.set(274877923330L);
        p("set_long", packed.equals(h124) + "," + packed + "," + new BlockPos.MutableBlockPos().set(-8194L));
        BlockPos.MutableBlockPos offset = new BlockPos.MutableBlockPos().setWithOffset(h123, 0, 0, 1);
        p("with_offset", offset.equals(h124) + "," + offset + "," + new HashSet<BlockPos>(List.of(h124)).contains(offset)
            + "," + new BlockPos.MutableBlockPos().setWithOffset(h123, Direction.SOUTH));
        // `toString` names the class as `getSimpleName` does: empty for an
        // anonymous class, the declared name for a local one.
        BlockPos anon = new BlockPos(1, 2, 3) {};
        class LocalPos extends BlockPos { LocalPos() { super(1, 2, 3); } }
        p("names", anon + "," + anon.toString() + "," + new LocalPos() + "," + anon.equals(h123));
        Odd odd = new Odd();
        Mover mover = new Mover();
        mover.bump();
        counted = odd.equals(h123) + "," + h123.equals(odd) + ";" + mover.equals(h124) + "," + mover.hashCode();
    }
}
