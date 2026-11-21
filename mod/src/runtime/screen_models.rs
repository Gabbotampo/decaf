//! Generic client container-screen bridge.
//!
//! Screen classes remain jar-owned; this module drives the real 26.3 screen
//! lifecycle (`MenuScreens$ScreenConstructor.create` -> `Gui.setScreen` ->
//! `Screen.init(int, int)` -> virtual `init()` -> `AbstractContainerScreen.init()`
//! / `ItemCombinerScreen.init()` -> `subInit()` -> `menu.addSlotListener(this)`
//! -> `extractBackground`) while letting every mod override run as bytecode and
//! counting anything unmodelled.

use crate::runtime::{
    Host, MethodRef, NativeError, NativeJar, Runtime, StdHost, Value,
    client_surface::{DrawOp, GUI_TEXTURED_PIPELINE},
    jdk_call::Site,
    menu_models::{
        ABSTRACT_CONTAINER_MENU, ABSTRACT_CONTAINER_SCREEN, ITEM_COMBINER_MENU,
        ITEM_COMBINER_SCREEN, MENU_TYPE, SCREEN,
    },
};

pub use crate::runtime::menu_models::{ScreenLayout, ScreenSlot, ScreenSlotKind};

const EXTRACT_BG_DESC: &str = "(Lnet/minecraft/client/gui/GuiGraphicsExtractor;IIF)V";
const EXTRACT_ERROR_DESC: &str = "(Lnet/minecraft/client/gui/GuiGraphicsExtractor;II)V";
const EXTRACT_TRANSPARENT_BG_DESC: &str = "(Lnet/minecraft/client/gui/GuiGraphicsExtractor;)V";
const SCREEN_CONSTRUCTOR: &str = "net/minecraft/client/gui/screens/MenuScreens$ScreenConstructor";
const SCREEN_CONSTRUCTOR_CREATE_DESC: &str = "(Lnet/minecraft/world/inventory/AbstractContainerMenu;Lnet/minecraft/world/entity/player/Inventory;Lnet/minecraft/network/chat/Component;)Lnet/minecraft/client/gui/screens/Screen;";
const BLIT_10_DESC: &str = "(Lcom/mojang/renderpearl/api/pipeline/RenderPipeline;Lnet/minecraft/resources/Identifier;IIFFIIII)V";

#[derive(Clone, Debug, PartialEq)]
pub struct ClientScreenLayout {
    pub layout: ScreenLayout,
}

impl<H: Host> Runtime<'_, H> {
    /// Invoke a `MenuScreens$ScreenConstructor` (a constructor method reference,
    /// a static or capturing lambda, a bound method reference, or a jar class
    /// implementing `MenuScreens$ScreenConstructor.create`).
    fn invoke_screen_constructor(
        &mut self,
        constructor: &Value,
        menu: Value,
        inventory: Value,
        title: Value,
    ) -> Result<Value, NativeError> {
        let args = [menu, inventory, title];
        if let Some(screen) = self.invoke_callback(constructor, &args)? {
            return Ok(screen);
        }
        if crate::runtime::jar_object::object_class(constructor).is_some() {
            return self.call_virtual(
                Site::Interface,
                SCREEN_CONSTRUCTOR,
                "create",
                SCREEN_CONSTRUCTOR_CREATE_DESC,
                constructor,
                &args,
            );
        }
        Err(NativeError::Metadata(
            "MenuScreens constructor is not a jar callback".into(),
        ))
    }

    /// Run `Gui.setScreen` offsets 234 (virtual `added()`) and 298-319
    /// (`screen.init(guiWidth, guiHeight)`).
    fn run_screen_init(
        &mut self,
        screen: &Value,
        gui_width: i32,
        gui_height: i32,
    ) -> Result<(), NativeError> {
        // 26.3 Gui.setScreen offset 234: screen.added() (Screen.added offset 0 is return).
        if let Some(owner) = self.jar_screen_override(screen, "added", "()V")? {
            let _ = self.invoke(&owner, "added", "()V", Some(screen.clone()), &[])?;
        }
        let _ = self.invoke_vanilla_screen_method(
            SCREEN,
            "init",
            "(II)V",
            screen,
            &[Value::Int(gui_width), Value::Int(gui_height)],
        )?;
        Ok(())
    }

    /// Run `Gui.setScreen`'s teardown of the previous screen (virtual `removed()`).
    fn run_screen_removed(&mut self, screen: &Value) -> Result<(), NativeError> {
        self.dispatch_screen_virtual(screen, "removed", "()V", &[])?;
        Ok(())
    }

    /// Run virtual `extractBackground(graphics, mouseX, mouseY, partialTick)` on `screen`.
    fn run_screen_extract_background(
        &mut self,
        screen: &Value,
        graphics: Value,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) -> Result<(), NativeError> {
        let args = [
            graphics,
            Value::Int(mouse_x),
            Value::Int(mouse_y),
            Value::Float(f64::from(partial_tick)),
        ];
        self.dispatch_screen_virtual(screen, "extractBackground", EXTRACT_BG_DESC, &args)?;
        Ok(())
    }

    /// The jar class overriding `(name, desc)` on `screen`, if any.
    fn jar_screen_override(
        &self,
        screen: &Value,
        name: &str,
        desc: &str,
    ) -> Result<Option<String>, NativeError> {
        let probe = MethodRef {
            owner: SCREEN.into(),
            name: name.into(),
            descriptor: desc.into(),
        };
        self.jvm_selected_jar_body(&probe, 0xb6, Some(screen))
    }

    /// Virtual dispatch of a `Screen` method on `screen`: runs the jar override
    /// if present; otherwise dispatches to the screen's modelled vanilla base
    /// class, or counts it if the vanilla base is unmodelled.
    fn dispatch_screen_virtual(
        &mut self,
        screen: &Value,
        name: &str,
        desc: &str,
        args: &[Value],
    ) -> Result<Value, NativeError> {
        if let Some(owner) = self.jar_screen_override(screen, name, desc)? {
            return self.invoke(&owner, name, desc, Some(screen.clone()), args);
        }
        let Some(class) = crate::runtime::jar_object::object_class(screen) else {
            let r = MethodRef {
                owner: SCREEN.into(),
                name: name.into(),
                descriptor: desc.into(),
            };
            return self.host.unknown_operand_call(&r);
        };
        let vanilla_base = self
            .vanilla_superclass(class)
            .unwrap_or_else(|| class.to_owned());
        if let Some(value) =
            self.invoke_vanilla_screen_method(&vanilla_base, name, desc, screen, args)?
        {
            return Ok(value);
        }
        let r = MethodRef {
            owner: vanilla_base,
            name: name.into(),
            descriptor: desc.into(),
        };
        self.host.unknown_operand_call(&r)
    }

    /// Execute one modelled vanilla method of `Screen`, `AbstractContainerScreen`,
    /// or `ItemCombinerScreen` (for both virtual dispatch when not overridden
    /// and `invokespecial` `super.*` calls from mod overrides).
    ///
    /// Returns `Ok(None)` when `vanilla_class` or `(name, desc)` is outside the
    /// modelled surface so the caller counts it as unknown.
    pub(super) fn invoke_vanilla_screen_method(
        &mut self,
        vanilla_class: &str,
        name: &str,
        desc: &str,
        screen: &Value,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        if !matches!(
            vanilla_class,
            SCREEN | ABSTRACT_CONTAINER_SCREEN | ITEM_COMBINER_SCREEN
        ) {
            return Ok(None);
        }
        match (name, desc) {
            // 26.3 Screen.init(int, int) (public final) offsets 0-72:
            //   0-7:   this.width = width; this.height = height;
            //   10-25: if (!this.initialized) { this.init(); this.setInitialFocus(); }
            //   28-29: else { this.repositionElements(); }
            //   32-34: this.initialized = true;
            //   37-72: triggerImmediateNarration / suppressNarration
            ("init", "(II)V") => {
                let [Value::Int(w), Value::Int(h)] = args else {
                    return Ok(None);
                };
                let width_ref = MethodRef {
                    owner: SCREEN.into(),
                    name: "width".into(),
                    descriptor: "I".into(),
                };
                let height_ref = MethodRef {
                    owner: SCREEN.into(),
                    name: "height".into(),
                    descriptor: "I".into(),
                };
                self.host
                    .menu_field_set(screen, &width_ref, &Value::Int(*w));
                self.host
                    .menu_field_set(screen, &height_ref, &Value::Int(*h));
                let initialized = self.host.menu_screen_initialized(screen).unwrap_or(false);
                if !initialized {
                    self.dispatch_screen_virtual(screen, "init", "()V", &[])?;
                    // Offset 21: Screen.setInitialFocus() offsets 0-10 checks
                    // minecraft.getLastInputType().isKeyboard(), which Gui.setScreen
                    // offsets 83-90 set to InputType.NONE (false). Run a jar override
                    // if present.
                    if let Some(owner) =
                        self.jar_screen_override(screen, "setInitialFocus", "()V")?
                    {
                        let _ = self.invoke(
                            &owner,
                            "setInitialFocus",
                            "()V",
                            Some(screen.clone()),
                            &[],
                        )?;
                    }
                } else if let Some(owner) =
                    self.jar_screen_override(screen, "repositionElements", "()V")?
                {
                    let _ = self.invoke(
                        &owner,
                        "repositionElements",
                        "()V",
                        Some(screen.clone()),
                        &[],
                    )?;
                } else {
                    // Screen.repositionElements (offsets 0-4) calls rebuildWidgets(),
                    // which is not modelled: count it explicitly.
                    let r = MethodRef {
                        owner: SCREEN.into(),
                        name: "repositionElements".into(),
                        descriptor: "()V".into(),
                    };
                    self.host.unknown_operand_call(&r)?;
                }
                self.host.set_menu_screen_initialized(screen, true);
                // Offset 39: virtual `triggerImmediateNarration(false)`.
                // 26.3 Screen.triggerImmediateNarration offsets 0-15 calls private
                // `shouldRunNarration()` (offsets 0-24: `DEBUG_UI_NARRATION ||
                // minecraft.getNarrator().isActive()`, false by default). Run any
                // jar override as bytecode; `super.triggerImmediateNarration` stays
                // unmodelled so calling super from an override is counted.
                if let Some(owner) =
                    self.jar_screen_override(screen, "triggerImmediateNarration", "(Z)V")?
                {
                    let _ = self.invoke(
                        &owner,
                        "triggerImmediateNarration",
                        "(Z)V",
                        Some(screen.clone()),
                        &[Value::Int(0)],
                    )?;
                }
                Ok(Some(Value::Void))
            }
            // 26.3 Screen.added() offset 0: return (allows `super.added()` from jar overrides).
            ("added", "()V") => Ok(Some(Value::Void)),
            ("removed", "()V") => match vanilla_class {
                // 26.3 ItemCombinerScreen.removed() offsets 0-15:
                //   0-1:  super.removed()  (AbstractContainerScreen.removed)
                //   4-12: ((ItemCombinerMenu) this.menu).removeSlotListener(this)
                ITEM_COMBINER_SCREEN => {
                    self.invoke_vanilla_screen_method(
                        ABSTRACT_CONTAINER_SCREEN,
                        "removed",
                        "()V",
                        screen,
                        &[],
                    )?;
                    let menu_ref = MethodRef {
                        owner: ABSTRACT_CONTAINER_SCREEN.into(),
                        name: "menu".into(),
                        descriptor: "Lnet/minecraft/world/inventory/AbstractContainerMenu;".into(),
                    };
                    let menu = self
                        .host
                        .menu_field(screen, &menu_ref)
                        .unwrap_or(Value::Ref(None));
                    let remove_listener_ref = MethodRef {
                        owner: ITEM_COMBINER_MENU.into(),
                        name: "removeSlotListener".into(),
                        descriptor: "(Lnet/minecraft/world/inventory/ContainerListener;)V".into(),
                    };
                    if menu == Value::Ref(None) {
                        return Err(crate::runtime::null_checks::invoke_on_null(&remove_listener_ref));
                    }
                    if let Some(owner) =
                        self.jvm_selected_jar_body(&remove_listener_ref, 0xb6, Some(&menu))?
                    {
                        let _ = self.invoke(
                            &owner,
                            "removeSlotListener",
                            "(Lnet/minecraft/world/inventory/ContainerListener;)V",
                            Some(menu),
                            std::slice::from_ref(screen),
                        )?;
                    } else if self.host.menu_remove_slot_listener(&menu, screen).is_none() {
                        self.host.unknown_operand_call(&remove_listener_ref)?;
                    }
                    Ok(Some(Value::Void))
                }
                // 26.3 AbstractContainerScreen.removed() offsets 0-25:
                //   0-10:  if (this.minecraft.player == null) return;
                //   11-25: this.menu.removed(this.minecraft.player);
                ABSTRACT_CONTAINER_SCREEN => {
                    if !self.host.client_has_player() {
                        return Ok(Some(Value::Void));
                    }
                    let menu_ref = MethodRef {
                        owner: ABSTRACT_CONTAINER_SCREEN.into(),
                        name: "menu".into(),
                        descriptor: "Lnet/minecraft/world/inventory/AbstractContainerMenu;".into(),
                    };
                    let menu = self
                        .host
                        .menu_field(screen, &menu_ref)
                        .unwrap_or(Value::Ref(None));
                    let removed_ref = MethodRef {
                        owner: ABSTRACT_CONTAINER_MENU.into(),
                        name: "removed".into(),
                        descriptor: "(Lnet/minecraft/world/entity/player/Player;)V".into(),
                    };
                    if menu == Value::Ref(None) {
                        return Err(crate::runtime::null_checks::invoke_on_null(&removed_ref));
                    }
                    let player = crate::runtime::game_object::client_value("client/player".into());
                    self.call_virtual(
                        Site::Virtual,
                        ABSTRACT_CONTAINER_MENU,
                        "removed",
                        "(Lnet/minecraft/world/entity/player/Player;)V",
                        &menu,
                        &[player],
                    )?;
                    Ok(Some(Value::Void))
                }
                // 26.3 Screen.removed() offset 0: return
                SCREEN => Ok(Some(Value::Void)),
                _ => Ok(None),
            },
            ("init", "()V") => match vanilla_class {
                // 26.3 ItemCombinerScreen.init() offsets 0-19:
                //   0-1:  super.init()  (AbstractContainerScreen.init)
                //   4-5:  this.subInit()
                //   8-16: ((ItemCombinerMenu) this.menu).addSlotListener(this)
                ITEM_COMBINER_SCREEN => {
                    self.invoke_vanilla_screen_method(
                        ABSTRACT_CONTAINER_SCREEN,
                        "init",
                        "()V",
                        screen,
                        &[],
                    )?;
                    self.dispatch_screen_virtual(screen, "subInit", "()V", &[])?;
                    let menu_ref = MethodRef {
                        owner: ABSTRACT_CONTAINER_SCREEN.into(),
                        name: "menu".into(),
                        descriptor: "Lnet/minecraft/world/inventory/AbstractContainerMenu;".into(),
                    };
                    let menu = self
                        .host
                        .menu_field(screen, &menu_ref)
                        .unwrap_or(Value::Ref(None));
                    let add_listener_ref = MethodRef {
                        owner: ITEM_COMBINER_MENU.into(),
                        name: "addSlotListener".into(),
                        descriptor: "(Lnet/minecraft/world/inventory/ContainerListener;)V".into(),
                    };
                    if menu == Value::Ref(None) {
                        return Err(crate::runtime::null_checks::invoke_on_null(&add_listener_ref));
                    }
                    if let Some(owner) =
                        self.jvm_selected_jar_body(&add_listener_ref, 0xb6, Some(&menu))?
                    {
                        let _ = self.invoke(
                            &owner,
                            "addSlotListener",
                            "(Lnet/minecraft/world/inventory/ContainerListener;)V",
                            Some(menu),
                            std::slice::from_ref(screen),
                        )?;
                    } else {
                        let menu_vanilla = crate::runtime::jar_object::object_class(&menu)
                            .and_then(|c| self.vanilla_superclass(c))
                            .unwrap_or_else(|| ITEM_COMBINER_MENU.to_owned());
                        if matches!(
                            menu_vanilla.as_str(),
                            ABSTRACT_CONTAINER_MENU | ITEM_COMBINER_MENU
                        ) {
                            self.invoke_menu_add_slot_listener(&menu, screen)?;
                        } else {
                            self.host.unknown_operand_call(&MethodRef {
                                owner: menu_vanilla,
                                ..add_listener_ref
                            })?;
                        }
                    }
                    Ok(Some(Value::Void))
                }
                // 26.3 AbstractContainerScreen.init() offsets 0-54:
                //   0-12:  this.leftPos = (this.width - this.imageWidth) / 2
                //   15-27: this.topPos = (this.height - this.imageHeight) / 2
                //   30-34: this.itemSlotMouseActions.clear()
                //   39-51: this.addItemSlotMouseAction(new BundleMouseActions(this.minecraft))
                ABSTRACT_CONTAINER_SCREEN => {
                    let width = self.screen_int_field(screen, SCREEN, "width")?;
                    let height = self.screen_int_field(screen, SCREEN, "height")?;
                    let image_width =
                        self.screen_int_field(screen, ABSTRACT_CONTAINER_SCREEN, "imageWidth")?;
                    let image_height =
                        self.screen_int_field(screen, ABSTRACT_CONTAINER_SCREEN, "imageHeight")?;
                    let left_pos = width.wrapping_sub(image_width) / 2;
                    let top_pos = height.wrapping_sub(image_height) / 2;
                    self.host.menu_field_set(
                        screen,
                        &MethodRef {
                            owner: ABSTRACT_CONTAINER_SCREEN.into(),
                            name: "leftPos".into(),
                            descriptor: "I".into(),
                        },
                        &Value::Int(left_pos),
                    );
                    self.host.menu_field_set(
                        screen,
                        &MethodRef {
                            owner: ABSTRACT_CONTAINER_SCREEN.into(),
                            name: "topPos".into(),
                            descriptor: "I".into(),
                        },
                        &Value::Int(top_pos),
                    );
                    // Offsets 39-51: AbstractContainerScreen.addItemSlotMouseAction (offsets 0-11)
                    // only appends to private itemSlotMouseActions; if a mod overrides it, count it.
                    if let Some(owner) = self.jar_screen_override(
                        screen,
                        "addItemSlotMouseAction",
                        "(Lnet/minecraft/client/gui/ItemSlotMouseAction;)V",
                    )? {
                        self.host.unknown_operand_call(&MethodRef {
                            owner,
                            name: "addItemSlotMouseAction".into(),
                            descriptor: "(Lnet/minecraft/client/gui/ItemSlotMouseAction;)V".into(),
                        })?;
                    }
                    Ok(Some(Value::Void))
                }
                // 26.3 Screen.init() offset 0: return
                SCREEN => Ok(Some(Value::Void)),
                _ => Ok(None),
            },
            // 26.3 ItemCombinerScreen.subInit() offset 0: return
            ("subInit", "()V") if vanilla_class == ITEM_COMBINER_SCREEN => Ok(Some(Value::Void)),
            ("extractBackground", EXTRACT_BG_DESC) => match vanilla_class {
                // 26.3 ItemCombinerScreen.extractBackground offsets 0-57:
                //   0-6:   super.extractBackground(graphics, mouseX, mouseY, partialTick)
                //   9-41:  graphics.blit(RenderPipelines.GUI_TEXTURED, this.menuResource,
                //                        this.leftPos, this.topPos, 0.0f, 0.0f,
                //                        this.imageWidth, this.imageHeight, 256, 256)
                //   44-54: this.extractErrorIcon(graphics, this.leftPos, this.topPos)
                ITEM_COMBINER_SCREEN => {
                    self.invoke_vanilla_screen_method(
                        ABSTRACT_CONTAINER_SCREEN,
                        "extractBackground",
                        EXTRACT_BG_DESC,
                        screen,
                        args,
                    )?;
                    let Some(graphics) = args.first() else {
                        return Ok(None);
                    };
                    let menu_resource = self
                        .host
                        .menu_field(
                            screen,
                            &MethodRef {
                                owner: ITEM_COMBINER_SCREEN.into(),
                                name: "menuResource".into(),
                                descriptor: "Lnet/minecraft/resources/Identifier;".into(),
                            },
                        )
                        .unwrap_or(Value::Ref(None));
                    let left_pos =
                        self.screen_int_field(screen, ABSTRACT_CONTAINER_SCREEN, "leftPos")?;
                    let top_pos =
                        self.screen_int_field(screen, ABSTRACT_CONTAINER_SCREEN, "topPos")?;
                    let image_width =
                        self.screen_int_field(screen, ABSTRACT_CONTAINER_SCREEN, "imageWidth")?;
                    let image_height =
                        self.screen_int_field(screen, ABSTRACT_CONTAINER_SCREEN, "imageHeight")?;
                    let blit_ref = MethodRef {
                        owner: "net/minecraft/client/gui/GuiGraphicsExtractor".into(),
                        name: "blit".into(),
                        descriptor: BLIT_10_DESC.into(),
                    };
                    if *graphics == Value::Ref(None) {
                        return Err(crate::runtime::null_checks::invoke_on_null(&blit_ref));
                    }
                    let pipeline =
                        crate::runtime::game_object::client_value(GUI_TEXTURED_PIPELINE.to_owned());
                    self.host.invoke_instance(
                        &blit_ref,
                        graphics,
                        &[
                            pipeline,
                            menu_resource,
                            Value::Int(left_pos),
                            Value::Int(top_pos),
                            Value::Float(0.0),
                            Value::Float(0.0),
                            Value::Int(image_width),
                            Value::Int(image_height),
                            Value::Int(256),
                            Value::Int(256),
                        ],
                    )?;
                    // Offsets 44-54: re-read leftPos/topPos and call virtual extractErrorIcon.
                    let error_left =
                        self.screen_int_field(screen, ABSTRACT_CONTAINER_SCREEN, "leftPos")?;
                    let error_top =
                        self.screen_int_field(screen, ABSTRACT_CONTAINER_SCREEN, "topPos")?;
                    self.dispatch_screen_virtual(
                        screen,
                        "extractErrorIcon",
                        EXTRACT_ERROR_DESC,
                        &[
                            graphics.clone(),
                            Value::Int(error_left),
                            Value::Int(error_top),
                        ],
                    )?;
                    Ok(Some(Value::Void))
                }
                // 26.3 Screen.extractBackground offsets 0-55 + AbstractContainerScreen.isInGameUi offsets 0-1:
                //   0-12:  if (this.isInGameUi()) this.extractTransparentBackground(graphics)
                //   15-41: else panorama / blurred / menu background (not host-owned)
                //   42-55: minecraft.gui.hud.extractDeferredSubtitles()
                // The host backdrop boundary holds iff this is an AbstractContainerScreen
                // (isInGameUi() == true) and the mod overrides neither isInGameUi() nor
                // extractTransparentBackground(GuiGraphicsExtractor).
                ABSTRACT_CONTAINER_SCREEN | SCREEN => {
                    if !self.host.menu_screen_is_container(screen) {
                        return Ok(None);
                    }
                    if let Some(owner) = self.jar_screen_override(screen, "isInGameUi", "()Z")? {
                        self.host.unknown_operand_call(&MethodRef {
                            owner,
                            name: "isInGameUi".into(),
                            descriptor: "()Z".into(),
                        })?;
                        return Ok(Some(Value::Void));
                    }
                    if let Some(owner) = self.jar_screen_override(
                        screen,
                        "extractTransparentBackground",
                        EXTRACT_TRANSPARENT_BG_DESC,
                    )? {
                        self.host.unknown_operand_call(&MethodRef {
                            owner,
                            name: "extractTransparentBackground".into(),
                            descriptor: EXTRACT_TRANSPARENT_BG_DESC.into(),
                        })?;
                        return Ok(Some(Value::Void));
                    }
                    Ok(Some(Value::Void))
                }
                _ => Ok(None),
            },
            // 26.3 ItemCombinerScreen.slotChanged (offset 0: return) and dataChanged (offset 0: return)
            (
                "slotChanged",
                "(Lnet/minecraft/world/inventory/AbstractContainerMenu;ILnet/minecraft/world/item/ItemStack;)V",
            )
            | ("dataChanged", "(Lnet/minecraft/world/inventory/AbstractContainerMenu;II)V")
                if vanilla_class == ITEM_COMBINER_SCREEN =>
            {
                Ok(Some(Value::Void))
            }
            _ => Ok(None),
        }
    }

    fn screen_int_field(
        &mut self,
        screen: &Value,
        owner: &str,
        name: &str,
    ) -> Result<i32, NativeError> {
        let r = MethodRef {
            owner: owner.into(),
            name: name.into(),
            descriptor: "I".into(),
        };
        match self.host.menu_field(screen, &r) {
            Some(Value::Int(v)) => Ok(v),
            _ => Err(NativeError::Metadata(format!(
                "missing screen int field {owner}#{name}"
            ))),
        }
    }
}

impl NativeJar {
    /// Construct a client-side menu through `MenuType.create(containerId, inventory)`
    /// (26.3 `MenuScreens$ScreenConstructor.fromPacket` offsets 0-11).
    pub fn create_client_menu(
        &self,
        host: &mut StdHost,
        menu_type: Value,
        container_id: i32,
    ) -> Result<Value, NativeError> {
        if matches!(menu_type, Value::Ref(None)) {
            return Err(NativeError::Metadata("menu type is null".into()));
        }
        let inventory = host.client_inventory_value();
        self.with_persistent_runtime(host, &[menu_type.clone(), inventory.clone()], |runtime| {
            let r = MethodRef {
                owner: MENU_TYPE.into(),
                name: "create".into(),
                descriptor: "(ILnet/minecraft/world/entity/player/Inventory;)Lnet/minecraft/world/inventory/AbstractContainerMenu;".into(),
            };
            runtime
                .invoke_menu_model(&r, &menu_type, &[Value::Int(container_id), inventory])?
                .ok_or_else(|| {
                    NativeError::Metadata("MenuType was not constructed with a MenuSupplier".into())
                })
        })
    }

    /// Construct a jar object (allocating a real `JarObject` handle) and invoke
    /// its constructor with `args` under persistent class state and resident heap.
    pub fn instantiate_jar_object(
        &self,
        class: &str,
        descriptor: &str,
        args: &[Value],
        host: &mut StdHost,
    ) -> Result<Value, NativeError> {
        let class_name = class.replace('.', "/");
        self.with_persistent_runtime(host, args, |runtime| {
            let object = runtime.new_object(&class_name);
            runtime.invoke(
                &class_name,
                "<init>",
                descriptor,
                Some(object.clone()),
                args,
            )?;
            Ok(object)
        })
    }

    /// Construct and initialize (`Screen.init(guiWidth, guiHeight)`) the screen
    /// registered for `menu_type`, tearing down any previously active screen via
    /// virtual `removed()`, and return its post-init layout.
    pub fn open_client_screen(
        &self,
        host: &mut StdHost,
        menu_type: Value,
        title: Value,
    ) -> Result<ClientScreenLayout, NativeError> {
        let [gui_w, gui_h] = host
            .client_gui_size()
            .ok_or_else(|| NativeError::Metadata("client frame is not active".into()))?;
        let constructor = host.client_menu_constructor(&menu_type).ok_or_else(|| {
            NativeError::Metadata("no MenuScreens registration for menu type".into())
        })?;
        let menu = host
            .client_menu_value_for_type(&menu_type)
            .ok_or_else(|| NativeError::Metadata("no constructed menu matches menu type".into()))?;
        let inventory = host.client_inventory_value();
        let previous = host.client_active_screen_value();
        let mut roots = vec![
            menu_type.clone(),
            constructor.clone(),
            menu.clone(),
            inventory.clone(),
            title.clone(),
        ];
        if let Some(prev) = &previous {
            roots.push(prev.clone());
        }
        // 26.3 order: `MenuScreens$ScreenConstructor.fromPacket` constructs the new screen
        // first (offset 22); if construction throws or returns null, the previous active
        // screen is left untouched. Only after non-null construction does `Gui.setScreen`
        // run `prev.removed()` (offset 77), switch active screen (offset 220), then
        // `added()` (offset 234) and `init(w, h)` (offsets 298-319).
        let screen = self.with_persistent_runtime(host, &roots, |runtime| {
            let screen = runtime.invoke_screen_constructor(
                &constructor,
                menu.clone(),
                inventory,
                title.clone(),
            )?;
            if matches!(screen, Value::Ref(None)) {
                return Err(NativeError::Metadata(
                    "MenuScreens constructor returned null".into(),
                ));
            }
            if let Some(prev) = previous {
                runtime.run_screen_removed(&prev)?;
            }
            runtime.host.set_client_active_screen_slot(
                menu_type.clone(),
                menu.clone(),
                title.clone(),
                [gui_w, gui_h],
                screen.clone(),
            );
            runtime.run_screen_init(&screen, gui_w, gui_h)?;
            Ok(screen)
        })?;
        let layout = host.client_screen_layout(&screen).ok_or_else(|| {
            NativeError::Metadata("screen constructor did not initialize a container screen".into())
        })?;
        Ok(ClientScreenLayout { layout })
    }

    /// Invoke `extractBackground` on the active screen for `menu_type` (reusing
    /// an already-open screen from `open_client_screen` for the same menu/title/size
    /// so no duplicate screen or leaked slot listener is created; otherwise tearing
    /// down any previous screen via `removed()`, constructing, and initializing),
    /// returning the recorded [`DrawOp`]s.
    pub fn draw_client_screen_bg(
        &self,
        host: &mut StdHost,
        menu_type: Value,
        title: Value,
    ) -> Result<Vec<DrawOp>, NativeError> {
        let [gui_w, gui_h] = host
            .client_gui_size()
            .ok_or_else(|| NativeError::Metadata("client frame is not active".into()))?;
        let constructor = host.client_menu_constructor(&menu_type).ok_or_else(|| {
            NativeError::Metadata("no MenuScreens registration for menu type".into())
        })?;
        let menu = host
            .client_menu_value_for_type(&menu_type)
            .ok_or_else(|| NativeError::Metadata("no constructed menu matches menu type".into()))?;
        let inventory = host.client_inventory_value();
        let before = host.client_draws().len();
        if let Some(existing) =
            host.client_matching_active_screen(&menu_type, &menu, &title, [gui_w, gui_h])
        {
            let root = existing.clone();
            self.with_persistent_runtime(host, std::slice::from_ref(&root), |runtime| {
                runtime.run_screen_extract_background(
                    &existing,
                    crate::runtime::game_object::client_value("client/gui-graphics".to_owned()),
                    0,
                    0,
                    0.0,
                )?;
                Ok(existing)
            })?;
            return Ok(host.client_draws()[before..].to_vec());
        }
        let previous = host.client_active_screen_value();
        let mut roots = vec![
            menu_type.clone(),
            constructor.clone(),
            menu.clone(),
            inventory.clone(),
            title.clone(),
        ];
        if let Some(prev) = &previous {
            roots.push(prev.clone());
        }
        let _screen = self.with_persistent_runtime(host, &roots, |runtime| {
            let screen = runtime.invoke_screen_constructor(
                &constructor,
                menu.clone(),
                inventory,
                title.clone(),
            )?;
            if matches!(screen, Value::Ref(None)) {
                return Err(NativeError::Metadata(
                    "MenuScreens constructor returned null".into(),
                ));
            }
            if let Some(prev) = previous {
                runtime.run_screen_removed(&prev)?;
            }
            runtime.host.set_client_active_screen_slot(
                menu_type.clone(),
                menu.clone(),
                title.clone(),
                [gui_w, gui_h],
                screen.clone(),
            );
            runtime.run_screen_init(&screen, gui_w, gui_h)?;
            runtime.run_screen_extract_background(
                &screen,
                crate::runtime::game_object::client_value("client/gui-graphics".to_owned()),
                0,
                0,
                0.0,
            )?;
            Ok(screen)
        })?;
        Ok(host.client_draws()[before..].to_vec())
    }
}
