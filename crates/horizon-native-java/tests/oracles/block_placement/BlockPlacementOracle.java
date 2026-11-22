import java.io.IOException;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.StringJoiner;
import java.util.function.Supplier;
import net.minecraft.SharedConstants;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.server.Bootstrap;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.item.context.DirectionalPlaceContext;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

/**
 * Java 25 ground-truth oracle for Minecraft 26.3 block placement contexts:
 * {@link Direction#orderedByNearest}, {@link UseOnContext}, {@link BlockPlaceContext},
 * and {@link DirectionalPlaceContext}.
 *
 * <p>Every value is produced directly by executing the real classes from
 * {@code server-26.3.jar}; no placement math is reimplemented here.
 *
 * <p>Writes {@code placement_java25.txt} into {@code args[0]}.
 */
public final class BlockPlacementOracle {
  private static final float[] YAWS = {
      0.0f,
      -0.0f,
      22.5f,
      45.0f,
      89.99f,
      90.0f,
      135.0f,
      180.0f,
      -180.0f,
      -135.0f,
      -90.0f,
      -45.0f,
      -157.3f,
      270.0f,
      360.0f,
      -360.0f,
      720.0f,
  };

  private static final float[] PITCHES = {
      -90.0f,
      -45.0f,
      -22.5f,
      -0.0f,
      0.0f,
      13.7f,
      45.0f,
      89.99f,
      90.0f,
  };

  private static final Direction[] FACES = {
      Direction.DOWN,
      Direction.UP,
      Direction.NORTH,
      Direction.SOUTH,
      Direction.WEST,
      Direction.EAST,
  };

  private static final Object UNSAFE;
  private static final Method M_ALLOCATE_INSTANCE;
  private static final Field F_PLAYER;
  private static final Field F_HAND;
  private static final Field F_HIT_RESULT;
  private static final Field F_ITEM_STACK;
  private static final Field F_RELATIVE_POS;
  private static final Field F_REPLACE_CLICKED;
  private static final Field F_DIRECTION;

  static {
    try {
      Class<?> unsafeClass = Class.forName("sun.misc.Unsafe");
      Field unsafeField = unsafeClass.getDeclaredField("theUnsafe");
      unsafeField.setAccessible(true);
      UNSAFE = unsafeField.get(null);
      M_ALLOCATE_INSTANCE = unsafeClass.getMethod("allocateInstance", Class.class);

      F_PLAYER = field(UseOnContext.class, "player");
      F_HAND = field(UseOnContext.class, "hand");
      F_HIT_RESULT = field(UseOnContext.class, "hitResult");
      F_ITEM_STACK = field(UseOnContext.class, "itemStack");
      F_RELATIVE_POS = field(BlockPlaceContext.class, "relativePos");
      F_REPLACE_CLICKED = field(BlockPlaceContext.class, "replaceClicked");
      F_DIRECTION = field(DirectionalPlaceContext.class, "direction");
    } catch (ReflectiveOperationException e) {
      throw new ExceptionInInitializerError(e);
    }
  }

  private static Field field(Class<?> owner, String name) throws NoSuchFieldException {
    Field f = owner.getDeclaredField(name);
    f.setAccessible(true);
    return f;
  }

  @SuppressWarnings("unchecked")
  private static <T> T allocate(Class<T> type) throws ReflectiveOperationException {
    return (T) M_ALLOCATE_INSTANCE.invoke(UNSAFE, type);
  }

  private static ServerPlayer newPlayer(float yaw, float pitch)
      throws ReflectiveOperationException {
    ServerPlayer player = allocate(ServerPlayer.class);
    player.setYRot(yaw);
    player.setYHeadRot(yaw);
    player.setXRot(pitch);
    if (Float.floatToRawIntBits(player.getViewYRot(1.0f)) != Float.floatToRawIntBits(player.getYRot())) {
      throw new IllegalStateException("viewYRot mismatch for yaw=" + yaw);
    }
    if (Float.floatToRawIntBits(player.getViewXRot(1.0f)) != Float.floatToRawIntBits(player.getXRot())) {
      throw new IllegalStateException("viewXRot mismatch for pitch=" + pitch);
    }
    return player;
  }

  private static BlockPlaceContext newBlockPlaceContext(
      Player player, BlockPos hitPos, Direction clickedFace, boolean replaceClicked)
      throws ReflectiveOperationException {
    BlockPlaceContext ctx = allocate(BlockPlaceContext.class);
    BlockHitResult hit = new BlockHitResult(Vec3.atCenterOf(hitPos), clickedFace, hitPos, false);
    F_PLAYER.set(ctx, player);
    F_HAND.set(ctx, InteractionHand.MAIN_HAND);
    F_HIT_RESULT.set(ctx, hit);
    F_ITEM_STACK.set(ctx, ItemStack.EMPTY);
    F_RELATIVE_POS.set(ctx, hitPos.relative(clickedFace));
    F_REPLACE_CLICKED.set(ctx, replaceClicked);
    return ctx;
  }

  private static DirectionalPlaceContext newDirectionalPlaceContext(
      BlockPos pos, Direction direction, Direction clickedFace)
      throws ReflectiveOperationException {
    DirectionalPlaceContext ctx = allocate(DirectionalPlaceContext.class);
    BlockHitResult hit =
        new BlockHitResult(Vec3.atBottomCenterOf(pos), clickedFace, pos, false);
    F_PLAYER.set(ctx, null);
    F_HAND.set(ctx, InteractionHand.MAIN_HAND);
    F_HIT_RESULT.set(ctx, hit);
    F_ITEM_STACK.set(ctx, ItemStack.EMPTY);
    F_RELATIVE_POS.set(ctx, pos.relative(clickedFace));
    F_REPLACE_CLICKED.set(ctx, true);
    F_DIRECTION.set(ctx, direction);
    return ctx;
  }

  private static String fmtDirs(Direction[] dirs) {
    StringJoiner joiner = new StringJoiner(",");
    for (Direction d : dirs) {
      joiner.add(d.name());
    }
    return joiner.toString();
  }

  private static String hexBits(float f) {
    return String.format("%08x", Float.floatToRawIntBits(f));
  }

  private static String captureEx(Supplier<Object> op) {
    try {
      Object v = op.get();
      if (v instanceof Direction[] arr) {
        return "OK:" + fmtDirs(arr);
      }
      return "OK:" + v;
    } catch (Throwable t) {
      return "EX:" + t.getClass().getName() + ":" + t.getMessage();
    }
  }

  public static void main(String[] args) throws Exception {
    if (args.length != 1) {
      throw new IllegalArgumentException("usage: BlockPlacementOracle <out-dir>");
    }
    Path outDir = Path.of(args[0]);
    Files.createDirectories(outDir);

    SharedConstants.tryDetectVersion();
    Bootstrap.bootStrap();

    BlockPos hitPos = new BlockPos(4, 5, 6);
    StringBuilder sb = new StringBuilder();
    sb.append("# Minecraft 26.3 BlockPlaceContext / DirectionalPlaceContext oracle (JDK 25)\n");

    // 1. Player look & rotation accessors across the full yaw x pitch grid.
    for (float yaw : YAWS) {
      for (float pitch : PITCHES) {
        ServerPlayer player = newPlayer(yaw, pitch);
        BlockPlaceContext ctx = newBlockPlaceContext(player, hitPos, Direction.UP, false);
        Direction[] ordered = Direction.orderedByNearest(player);
        Direction horiz = ctx.getHorizontalDirection();
        float rot = ctx.getRotation();
        Direction nearest = ctx.getNearestLookingDirection();
        Direction nearestVert = ctx.getNearestLookingVerticalDirection();
        sb.append("PLAYER|")
            .append(hexBits(player.getYRot()))
            .append('|')
            .append(hexBits(player.getXRot()))
            .append('|')
            .append(horiz.name())
            .append('|')
            .append(hexBits(rot))
            .append('|')
            .append(nearest.name())
            .append('|')
            .append(nearestVert.name())
            .append('|')
            .append(fmtDirs(ordered))
            .append('\n');
      }
    }

    // 2. BlockPlaceContext.getNearestLookingDirections across yaw x pitch x 6 faces x replaceClicked.
    for (float yaw : YAWS) {
      for (float pitch : PITCHES) {
        ServerPlayer player = newPlayer(yaw, pitch);
        for (Direction face : FACES) {
          for (boolean replaceClicked : new boolean[] {false, true}) {
            BlockPlaceContext ctx = newBlockPlaceContext(player, hitPos, face, replaceClicked);
            Direction[] dirs = ctx.getNearestLookingDirections();
            sb.append("PLACE_DIRS|")
                .append(hexBits(player.getYRot()))
                .append('|')
                .append(hexBits(player.getXRot()))
                .append('|')
                .append(face.name())
                .append('|')
                .append(replaceClicked)
                .append('|')
                .append(fmtDirs(dirs))
                .append('\n');
          }
        }
      }
    }

    // 3. Plain BlockPlaceContext with null player across 6 faces x replaceClicked.
    for (Direction face : FACES) {
      for (boolean replaceClicked : new boolean[] {false, true}) {
        BlockPlaceContext ctx = newBlockPlaceContext(null, hitPos, face, replaceClicked);
        sb.append("NULL_PLAYER|")
            .append(face.name())
            .append('|')
            .append(replaceClicked)
            .append('|')
            .append(ctx.getPlayer() == null)
            .append('|')
            .append(ctx.getHorizontalDirection().name())
            .append('|')
            .append(hexBits(ctx.getRotation()))
            .append('|')
            .append(ctx.isSecondaryUseActive())
            .append('|')
            .append(captureEx(ctx::getNearestLookingDirection))
            .append('|')
            .append(captureEx(ctx::getNearestLookingVerticalDirection))
            .append('|')
            .append(captureEx(ctx::getNearestLookingDirections))
            .append('\n');
      }
    }

    // 4. DirectionalPlaceContext across all 6 dispenser directions x 6 clicked faces.
    for (Direction dir : FACES) {
      for (Direction face : FACES) {
        DirectionalPlaceContext ctx = newDirectionalPlaceContext(hitPos, dir, face);
        sb.append("DIRECTIONAL|")
            .append(dir.name())
            .append('|')
            .append(face.name())
            .append('|')
            .append(ctx.getPlayer() == null)
            .append('|')
            .append(ctx.getHorizontalDirection().name())
            .append('|')
            .append(hexBits(ctx.getRotation()))
            .append('|')
            .append(ctx.isSecondaryUseActive())
            .append('|')
            .append(ctx.getNearestLookingDirection().name())
            .append('|')
            .append(captureEx(ctx::getNearestLookingVerticalDirection))
            .append('|')
            .append(fmtDirs(ctx.getNearestLookingDirections()))
            .append('\n');
      }
    }

    Path outFile = outDir.resolve("placement_java25.txt");
    Files.writeString(outFile, sb.toString(), StandardCharsets.UTF_8);
    long rows = Arrays.stream(sb.toString().split("\n")).filter(l -> !l.startsWith("#")).count();
    System.out.println("Wrote " + rows + " oracle rows to " + outFile);
  }
}
