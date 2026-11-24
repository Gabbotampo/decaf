package demo;

import net.minecraft.world.entity.player.Inventory;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.inventory.ContainerLevelAccess;
import net.minecraft.world.inventory.ItemCombinerMenu;
import net.minecraft.world.inventory.ItemCombinerMenuSlotDefinition;
import net.minecraft.world.inventory.MenuType;
import net.minecraft.world.inventory.Slot;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.level.block.state.BlockState;

/**
 * A two-input combiner: cobblestone + amethyst shard make 4 stone bricks. Taking the result
 * shrinks each input by one. Its second input refuses dirt; pick-all skips the result slot.
 */
public class BenchMenu extends ItemCombinerMenu {
  public static final MenuType<BenchMenu> TYPE = Types.bench();

  public BenchMenu(int id, Inventory inventory) {
    this(id, inventory, ContainerLevelAccess.NULL);
  }

  public BenchMenu(int id, Inventory inventory, ContainerLevelAccess access) {
    super(TYPE, id, inventory, access, slots());
  }

  private static ItemCombinerMenuSlotDefinition slots() {
    return ItemCombinerMenuSlotDefinition.create()
        .withSlot(0, 27, 47, stack -> true)
        .withSlot(1, 76, 47, stack -> !stack.is(Items.DIRT))
        .withResultSlot(2, 134, 47)
        .build();
  }

  @Override
  protected boolean isValidBlock(BlockState state) {
    return state.is(Probe.BENCH);
  }

  @Override
  public void createResult() {
    ItemStack base = inputSlots.getItem(0);
    ItemStack addition = inputSlots.getItem(1);
    System.out.println("mod: createResult " + base.toString() + " + " + addition.toString());
    if (base.is(Items.COBBLESTONE) && addition.is(Items.AMETHYST_SHARD)) {
      resultSlots.setItem(0, new ItemStack(Items.STONE_BRICKS, 4));
    } else {
      resultSlots.setItem(0, ItemStack.EMPTY);
    }
  }

  @Override
  protected void onTake(Player player, ItemStack stack) {
    System.out.println("mod: onTake " + stack.toString());
    stack.onCraftedBy(player, stack.getCount());
    shrink(0);
    shrink(1);
  }

  private void shrink(int index) {
    ItemStack stack = inputSlots.getItem(index);
    if (!stack.isEmpty()) {
      stack.shrink(1);
      inputSlots.setItem(index, stack);
    }
  }

  @Override
  public boolean canTakeItemForPickAll(ItemStack stack, Slot slot) {
    return slot.container != resultSlots && super.canTakeItemForPickAll(stack, slot);
  }
}
