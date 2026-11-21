import java.lang.reflect.*;
import java.util.*;
import java.util.jar.*;
import net.minecraft.SharedConstants;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.*;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.shapes.*;

public final class CaptureAllOracle {
    static String fmtAabbs(VoxelShape s) {
        StringBuilder sb = new StringBuilder("[");
        boolean first = true;
        for (AABB a : s.toAabbs()) {
            if (!first) sb.append(", ");
            first = false;
            sb.append(String.format(Locale.ROOT, "[%s, %s, %s, %s, %s, %s]",
                a.minX, a.minY, a.minZ, a.maxX, a.maxY, a.maxZ));
        }
        return sb.append("]").toString();
    }

    public static void main(String[] args) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();

        // 1. Capture BlockStateProperties value class + enum constant names
        System.out.println("=== BLOCK_STATE_PROPERTY_VALUES ===");
        Field[] fields = BlockStateProperties.class.getFields();
        Arrays.sort(fields, Comparator.comparing(Field::getName));
        Set<String> enumClasses = new TreeSet<>();
        for (Field f : fields) {
            if (!Modifier.isStatic(f.getModifiers()) || !Property.class.isAssignableFrom(f.getType())) continue;
            Property<?> prop = (Property<?>) f.get(null);
            Class<?> valCls = prop.getValueClass();
            if (valCls.isEnum()) {
                String clsInternal = valCls.getName().replace('.', '/');
                enumClasses.add(clsInternal);
                List<String> pairs = new ArrayList<>();
                for (Object v : prop.getPossibleValues()) {
                    @SuppressWarnings({"unchecked", "rawtypes"})
                    String ser = ((Property) prop).getName((Comparable) v);
                    String cname = ((Enum<?>) v).name();
                    pairs.add(ser + "=" + cname);
                }
                System.out.println(f.getName() + "|" + prop.getName() + "|" + clsInternal + "|" + String.join(",", pairs));
            }
        }
        System.out.println("=== ENUM_CLASSES_IN_PROPERTIES ===");
        for (String cls : enumClasses) {
            Class<?> c = Class.forName(cls.replace('/', '.'));
            Object[] consts = c.getEnumConstants();
            List<String> names = new ArrayList<>();
            for (Object o : consts) names.add(((Enum<?>) o).name());
            System.out.println(cls + "|" + String.join(",", names));
        }

        // 2. Capture vanilla Block classes getShape and getCollisionShape declaring classes
        System.out.println("=== VANILLA_BLOCK_GET_SHAPE ===");
        String mcJar = args[0];
        List<String> rows = new ArrayList<>();
        int shapeOverrides = 0;
        int collisionOverrides = 0;
        try (JarFile jf = new JarFile(mcJar)) {
            Enumeration<JarEntry> en = jf.entries();
            while (en.hasMoreElements()) {
                String name = en.nextElement().getName();
                if (!name.endsWith(".class") || name.contains("$")) continue;
                String clsName = name.substring(0, name.length() - 6).replace('/', '.');
                Class<?> c;
                try {
                    c = Class.forName(clsName, false, CaptureAllOracle.class.getClassLoader());
                } catch (Throwable ignored) { continue; }
                if (c.isInterface() || !Block.class.isAssignableFrom(c)) continue;
                String shapeDecl = "";
                for (Class<?> k = c; k != null && k != net.minecraft.world.level.block.state.BlockBehaviour.class; k = k.getSuperclass()) {
                    try {
                        k.getDeclaredMethod("getShape", BlockState.class, BlockGetter.class, BlockPos.class, CollisionContext.class);
                        shapeDecl = k.getName().replace('.', '/');
                        break;
                    } catch (NoSuchMethodException ignored) {}
                }
                String collisionDecl = "";
                for (Class<?> k = c; k != null && k != net.minecraft.world.level.block.state.BlockBehaviour.class; k = k.getSuperclass()) {
                    try {
                        k.getDeclaredMethod("getCollisionShape", BlockState.class, BlockGetter.class, BlockPos.class, CollisionContext.class);
                        collisionDecl = k.getName().replace('.', '/');
                        break;
                    } catch (NoSuchMethodException ignored) {}
                }
                if (!shapeDecl.isEmpty()) shapeOverrides++;
                if (!collisionDecl.isEmpty()) collisionOverrides++;
                rows.add(name.substring(0, name.length() - 6) + "|" + shapeDecl + "|" + collisionDecl);
            }
        }
        Collections.sort(rows);
        System.out.println("COUNT=" + rows.size());
        System.out.println("SHAPE_OVERRIDES=" + shapeOverrides + " SHAPE_INHERITS=" + (rows.size() - shapeOverrides));
        System.out.println("COLLISION_OVERRIDES=" + collisionOverrides + " COLLISION_INHERITS=" + (rows.size() - collisionOverrides));
        java.nio.file.Files.write(java.nio.file.Path.of(args[1]), rows);

        // 3. Test VoxelShape identity, canonicalization, equals, toString, Shapes.joinUnoptimized, move, optimize
        System.out.println("=== VOXEL_SHAPE_DETAILS ===");
        VoxelShape a1 = Block.box(0, 0, 0, 8, 8, 8);
        VoxelShape a2 = Block.box(0, 0, 0, 8, 8, 8);
        System.out.println("same_box_eq=" + a1.equals(a2) + " same_box_ref=" + (a1 == a2));
        System.out.println("empty_eq=" + Shapes.empty().equals(Shapes.empty()) + " ref=" + (Shapes.empty() == Shapes.empty()));
        System.out.println("block_eq=" + Shapes.block().equals(Shapes.block()) + " ref=" + (Shapes.block() == Shapes.block()));
        System.out.println("box_full_ref=" + (Block.box(0,0,0,16,16,16) == Shapes.block()));
        System.out.println("box_zero_ref=" + (Block.box(0,0,0,0,0,0) == Shapes.empty()));
        System.out.println("empty_toString=" + Shapes.empty().toString());
        System.out.println("block_toString=" + Shapes.block().toString());
        System.out.println("a1_toString=" + a1.toString());
        System.out.println("a1_class=" + a1.getClass().getName());
        System.out.println("empty_class=" + Shapes.empty().getClass().getName());
        System.out.println("block_class=" + Shapes.block().getClass().getName());
        System.out.println("non_grid_class=" + Shapes.box(0.1, 0.2, 0.3, 0.9, 0.8, 0.7).getClass().getName());

        VoxelShape half1 = Block.box(0, 0, 0, 8, 16, 16);
        VoxelShape half2 = Block.box(8, 0, 0, 16, 16, 16);
        VoxelShape unoptFull = Shapes.joinUnoptimized(half1, half2, BooleanOp.OR);
        VoxelShape optFull = Shapes.or(half1, half2);
        VoxelShape unoptEmpty = Shapes.joinUnoptimized(half1, half2, BooleanOp.AND);
        VoxelShape optEmpty = Shapes.join(half1, half2, BooleanOp.AND);
        System.out.println("unopt_full_is_block=" + (unoptFull == Shapes.block()) + " class=" + unoptFull.getClass().getName() + " aabbs=" + fmtAabbs(unoptFull) + " toString=" + unoptFull);
        System.out.println("opt_full_is_block=" + (optFull == Shapes.block()) + " class=" + optFull.getClass().getName());
        System.out.println("unopt_empty_is_empty_ref=" + (unoptEmpty == Shapes.empty()) + " isEmpty=" + unoptEmpty.isEmpty() + " class=" + unoptEmpty.getClass().getName() + " toString=" + unoptEmpty);
        System.out.println("opt_empty_is_empty_ref=" + (optEmpty == Shapes.empty()) + " isEmpty=" + optEmpty.isEmpty() + " class=" + optEmpty.getClass().getName());
        System.out.println("unopt_same_or_ref=" + (Shapes.joinUnoptimized(half1, half1, BooleanOp.OR) == half1));
        System.out.println("unopt_same_only_first_empty=" + (Shapes.joinUnoptimized(half1, half1, BooleanOp.ONLY_FIRST) == Shapes.empty()));
        System.out.println("unopt_empty_or_ref=" + (Shapes.joinUnoptimized(Shapes.empty(), half1, BooleanOp.OR) == half1));
        System.out.println("unopt_or_empty_ref=" + (Shapes.joinUnoptimized(half1, Shapes.empty(), BooleanOp.OR) == half1));
        System.out.println("unopt_empty_and_empty=" + (Shapes.joinUnoptimized(Shapes.empty(), half1, BooleanOp.AND) == Shapes.empty()));
        System.out.println("unopt_and_empty_empty=" + (Shapes.joinUnoptimized(half1, Shapes.empty(), BooleanOp.AND) == Shapes.empty()));
        System.out.println("block_move_zero_is_block=" + (Shapes.block().move(0, 0, 0) == Shapes.block()) + " class=" + Shapes.block().move(0, 0, 0).getClass().getName());
        System.out.println("empty_move_is_empty=" + (Shapes.empty().move(1, 2, 3) == Shapes.empty()));
        System.out.println("block_opt_is_block=" + (Shapes.block().optimize() == Shapes.block()));
        System.out.println("empty_opt_is_empty=" + (Shapes.empty().optimize() == Shapes.empty()));
        System.out.println("half1_opt_is_same=" + (half1.optimize() == half1));
        try {
            Shapes.joinUnoptimized(half1, half2, BooleanOp.TRUE);
        } catch (Throwable t) {
            System.out.println("join_true_throws=" + t.getClass().getName() + ":" + t.getMessage());
        }

        // 4. Test fixture shapes (static Shapes.or + facing-dependent shapes + no override)
        VoxelShape fStatic = Shapes.or(Block.box(2, 0, 2, 14, 4, 14), Block.box(5, 4, 5, 11, 12, 11));
        VoxelShape fNorth = Block.box(2, 0, 0, 14, 12, 6);
        VoxelShape fSouth = Block.box(2, 0, 10, 14, 12, 16);
        VoxelShape fWest  = Block.box(0, 0, 2, 6, 12, 14);
        VoxelShape fEast  = Block.box(10, 0, 2, 16, 12, 14);
        System.out.println("fixture_static=" + fmtAabbs(fStatic));
        System.out.println("fixture_north=" + fmtAabbs(fNorth));
        System.out.println("fixture_south=" + fmtAabbs(fSouth));
        System.out.println("fixture_west=" + fmtAabbs(fWest));
        System.out.println("fixture_east=" + fmtAabbs(fEast));

        // 5. BooleanOp.AND / ONLY_FIRST / ONLY_SECOND / NOT_SAME
        VoxelShape b1 = Block.box(0, 0, 0, 10, 10, 10);
        VoxelShape b2 = Block.box(6, 0, 0, 16, 10, 10);
        System.out.println("join_and=" + fmtAabbs(Shapes.join(b1, b2, BooleanOp.AND)));
        System.out.println("join_only_first=" + fmtAabbs(Shapes.join(b1, b2, BooleanOp.ONLY_FIRST)));
        System.out.println("join_only_second=" + fmtAabbs(Shapes.join(b1, b2, BooleanOp.ONLY_SECOND)));
        System.out.println("join_not_same=" + fmtAabbs(Shapes.join(b1, b2, BooleanOp.NOT_SAME)));

        // 6. StateHolder.setValue / getValue / trySetValue validation & Property.toString
        System.out.println("=== STATE_HOLDER_VALIDATION ===");
        System.out.println("prop_horizontal_facing=" + BlockStateProperties.HORIZONTAL_FACING);
        System.out.println("prop_lit=" + BlockStateProperties.LIT);
        System.out.println("prop_age_3=" + BlockStateProperties.AGE_3);
        System.out.println("prop_piston_type=" + BlockStateProperties.PISTON_TYPE);
        BlockState furnaceState = net.minecraft.world.level.block.Blocks.FURNACE.defaultBlockState();
        System.out.println("furnace_block_toString=" + furnaceState.getBlock());
        try {
            furnaceState.setValue(BlockStateProperties.WATERLOGGED, Boolean.TRUE);
        } catch (Throwable t) {
            System.out.println("set_missing_prop=" + t.getClass().getName() + ":" + t.getMessage());
        }
        try {
            furnaceState.setValue(BlockStateProperties.HORIZONTAL_FACING, Direction.UP);
        } catch (Throwable t) {
            System.out.println("set_bad_val=" + t.getClass().getName() + ":" + t.getMessage());
        }
        try {
            furnaceState.getValue(BlockStateProperties.WATERLOGGED);
        } catch (Throwable t) {
            System.out.println("get_missing_prop=" + t.getClass().getName() + ":" + t.getMessage());
        }
        System.out.println("try_set_missing_same=" + (furnaceState.trySetValue(BlockStateProperties.WATERLOGGED, Boolean.TRUE) == furnaceState));
        try {
            furnaceState.trySetValue(BlockStateProperties.HORIZONTAL_FACING, Direction.UP);
        } catch (Throwable t) {
            System.out.println("try_set_bad_val=" + t.getClass().getName() + ":" + t.getMessage());
        }
        // Check whether any BlockStateProperties enum constant has toString() != getSerializedName()
        System.out.println("=== ENUM_TO_STRING_DIFFS ===");
        for (String cls : enumClasses) {
            Class<?> c = Class.forName(cls.replace('/', '.'));
            for (Object o : c.getEnumConstants()) {
                String ts = o.toString();
                String ser = (o instanceof net.minecraft.util.StringRepresentable sr) ? sr.getSerializedName() : ((Enum<?>) o).name();
                if (!ts.equals(ser)) {
                    System.out.println("diff|" + cls + "|" + ((Enum<?>) o).name() + "|toString=" + ts + "|ser=" + ser);
                }
            }
        }
    }
}
