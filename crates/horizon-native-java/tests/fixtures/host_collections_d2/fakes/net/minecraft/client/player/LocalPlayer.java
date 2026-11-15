package net.minecraft.client.player;

import net.minecraft.world.entity.player.Inventory;

public class LocalPlayer {
    private final Inventory inventory = new Inventory();

    public Inventory getInventory() {
        return inventory;
    }
}
