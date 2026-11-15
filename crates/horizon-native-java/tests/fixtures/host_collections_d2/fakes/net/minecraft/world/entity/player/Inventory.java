package net.minecraft.world.entity.player;

import net.minecraft.core.NonNullList;

/** `Inventory.items`: `NonNullList.withSize(36, ItemStack.EMPTY)` in 26.3; distinct elements here, so a lost write shows. */
public class Inventory {
    private final NonNullList<Object> items = NonNullList.withSize(36, "");

    public Inventory() {
        for (int i = 0; i < 36; i++) {
            items.set(i, new String("stack" + i));
        }
    }

    public NonNullList<Object> getNonEquipmentItems() {
        return items;
    }
}
