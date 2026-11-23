import java.lang.reflect.Method;
import java.util.*;
import java.util.jar.*;
import net.minecraft.SharedConstants;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.EmptyBlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.shapes.BooleanOp;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;

public final class ShapeOracle {
    static String fmt(VoxelShape s) {
        StringBuilder sb = new StringBuilder();
        sb.append("empty=").append(s.isEmpty()).append(" aabbs=[");
        boolean first = true;
        for (AABB a : s.toAabbs()) {
            if (!first) sb.append(", ");
            first = false;
            sb.append(String.format(Locale.ROOT, "[%s,%s,%s,%s,%s,%s]",
                a.minX, a.minY, a.minZ, a.maxX, a.maxY, a.maxZ));
        }
        sb.append("]");
        if (!s.isEmpty()) {
            AABB b = s.bounds();
            sb.append(String.format(Locale.ROOT, " bounds=[%s,%s,%s,%s,%s,%s]",
                b.minX, b.minY, b.minZ, b.maxX, b.maxY, b.maxZ));
        }
        return sb.toString();
    }

    public static void main(String[] args) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();

        // 1. Runes altar exact constants from javap offsets 85..166:
        VoxelShape top = Block.box(1.0, 12.0, 1.0, 15.0, 16.0, 15.0);
        VoxelShape mid = Block.box(4.0, 3.0, 4.0, 12.0, 12.0, 12.0);
        VoxelShape bot = Block.box(1.0, 0.0, 1.0, 15.0, 3.0, 15.0);
        VoxelShape altar = Shapes.or(top, mid, bot);
        System.out.println("altar_top=" + fmt(top));
        System.out.println("altar_mid=" + fmt(mid));
        System.out.println("altar_bot=" + fmt(bot));
        System.out.println("altar_or=" + fmt(altar));
        System.out.println("altar_or_opt=" + fmt(altar.optimize()));

        // 2. Shapes primitives & edge cases
        System.out.println("empty=" + fmt(Shapes.empty()));
        System.out.println("block=" + fmt(Shapes.block()));
        System.out.println("thin=" + fmt(Shapes.box(0.0, 0.0, 0.0, 0.0, 1.0, 1.0)));
        System.out.println("eps_below=" + fmt(Shapes.box(0.0, 0.0, 0.0, 9.9e-8, 1.0, 1.0)));
        System.out.println("eps_at=" + fmt(Shapes.box(0.0, 0.0, 0.0, 1.0e-7, 1.0, 1.0)));
        System.out.println("non_grid=" + fmt(Shapes.box(0.1, 0.2, 0.3, 0.9, 0.8, 0.7)));
        System.out.println("out_of_unit=" + fmt(Block.box(-2, 0, -2, 18, 24, 18)));
        System.out.println("overlap=" + fmt(Shapes.or(Block.box(0, 0, 0, 10, 10, 10), Block.box(6, 0, 0, 16, 10, 10))));
        System.out.println("touching=" + fmt(Shapes.or(Block.box(0, 0, 0, 8, 16, 16), Block.box(8, 0, 0, 16, 16, 16))));
        System.out.println("l_shape=" + fmt(Shapes.or(Block.box(0, 0, 0, 16, 8, 16), Block.box(0, 8, 0, 8, 16, 16))));
        System.out.println("move=" + fmt(Block.box(0, 0, 0, 8, 8, 8).move(0.5, 0.25, 0.0)));
        System.out.println("or_empty=" + fmt(Shapes.or(Shapes.empty(), top)));
        System.out.println("or_varargs_empty=" + fmt(Shapes.or(top)));

        // 3. Invalid box exceptions
        for (String label : new String[]{"min_gt_max", "nan"}) {
            try {
                if (label.equals("min_gt_max")) Shapes.box(1.0, 0.0, 0.0, 0.5, 1.0, 1.0);
                else {
                    VoxelShape nan = Shapes.box(Double.NaN, 0.0, 0.0, 1.0, 1.0, 1.0);
                    System.out.println("nan_ok=" + fmt(nan));
                }
            } catch (Throwable t) {
                System.out.println(label + "_throws=" + t.getClass().getName() + ":" + t.getMessage());
            }
        }
        try {
            Shapes.empty().bounds();
        } catch (Throwable t) {
            System.out.println("empty_bounds_throws=" + t.getClass().getName() + ":" + t.getMessage());
        }

        // 4. Direction values
        for (Direction d : Direction.values()) {
            System.out.println("dir=" + d.name() + " ord=" + d.ordinal()
                + " 3d=" + d.get3DDataValue() + " 2d=" + d.get2DDataValue()
                + " opp=" + d.getOpposite().name() + " axis=" + d.getAxis().name()
                + " cw=" + (d.getAxis().isHorizontal() ? d.getClockWise().name() : "V")
                + " ccw=" + (d.getAxis().isHorizontal() ? d.getCounterClockWise().name() : "V")
                + " step=" + d.getStepX() + "," + d.getStepY() + "," + d.getStepZ()
                + " ser=" + d.getSerializedName());
        }

        // 5. Check which vanilla Block subclasses override getShape!
        String mcJar = args[0];
        int totalBlocks = 0, overrides = 0, inherits = 0;
        List<String> overriders = new ArrayList<>();
        try (JarFile jf = new JarFile(mcJar)) {
            Enumeration<JarEntry> en = jf.entries();
            while (en.hasMoreElements()) {
                String name = en.nextElement().getName();
                if (!name.startsWith("net/minecraft/world/level/block/") || !name.endsWith(".class") || name.contains("$")) continue;
                String clsName = name.substring(0, name.length() - 6).replace('/', '.');
                Class<?> c = Class.forName(clsName, false, ShapeOracle.class.getClassLoader());
                if (!Block.class.isAssignableFrom(c)) continue;
                totalBlocks++;
                Method m = null;
                for (Class<?> cur = c; cur != null; cur = cur.getSuperclass()) {
                    try {
                        m = cur.getDeclaredMethod("getShape", BlockState.class, BlockGetter.class, BlockPos.class, CollisionContext.class);
                        break;
                    } catch (NoSuchMethodException ignored) {}
                }
                String decl = m == null ? "NONE" : m.getDeclaringClass().getName().replace('.', '/');
                if (decl.equals("net/minecraft/world/level/block/state/BlockBehaviour")) {
                    inherits++;
                } else {
                    overrides++;
                    overriders.add(clsName.replace('.', '/') + "|" + decl);
                }
            }
        }
        Collections.sort(overriders);
        System.out.println("vanilla_blocks total=" + totalBlocks + " inherits_BlockBehaviour=" + inherits + " overrides_getShape=" + overrides);
        for (int i = 0; i < Math.min(20, overriders.size()); i++) {
            System.out.println("  override_sample: " + overriders.get(i));
        }
    }
}
