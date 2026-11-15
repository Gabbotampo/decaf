package net.minecraft.client;

import net.minecraft.client.player.LocalPlayer;

/** The game's client, as far as `demo.Inv` reaches it (Java side: a fake; Horizon: the host's). */
public class Minecraft {
    private static final Minecraft INSTANCE = new Minecraft();
    public final LocalPlayer player = new LocalPlayer();

    public static Minecraft getInstance() {
        return INSTANCE;
    }
}
