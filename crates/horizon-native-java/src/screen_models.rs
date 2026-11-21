//! Generic client container-screen bridge. The screen class remains jar-owned;
//! this module only exposes the vanilla fields and recorded graphics calls.

use crate::{
    NativeError, NativeJar, StdHost, Value, client_surface::DrawOp, events::ListenerCallback,
    menu_models::ScreenLayout,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ClientScreenLayout {
    pub layout: ScreenLayout,
}

fn callback(value: Value) -> Result<ListenerCallback, NativeError> {
    let row = crate::callback_marker::lambda_of(&value).ok_or_else(|| {
        NativeError::Metadata("MenuScreens constructor is not a jar callback".into())
    })?;
    Ok(ListenerCallback::Indy {
        target: row.target,
        captures: row.captures,
    })
}

fn construct(
    jar: &NativeJar,
    host: &mut StdHost,
    constructor: Value,
    menu: Value,
    inventory: Value,
    title: Value,
) -> Result<Value, NativeError> {
    let callback = callback(constructor)?;
    let target = match callback {
        ListenerCallback::Indy { target, .. } => target,
        _ => {
            return Err(NativeError::Metadata(
                "MenuScreens constructor is not an indy constructor".into(),
            ));
        }
    };
    let screen = Value::Ref(Some(format!("{}#client-screen", target.owner)));
    jar.invoke_method_with_args(
        &target.owner,
        "<init>",
        &target.descriptor,
        Some(screen.clone()),
        &[menu, inventory, title],
        host,
    )?;
    Ok(screen)
}

impl NativeJar {
    pub fn open_client_screen(
        &self,
        host: &mut StdHost,
        menu_type_id: Value,
        title: Value,
    ) -> Result<ClientScreenLayout, NativeError> {
        let constructor = host.client_menu_constructor(&menu_type_id).ok_or_else(|| {
            NativeError::Metadata("no MenuScreens registration for menu type".into())
        })?;
        let menu = host.client_menu_value().ok_or_else(|| {
            NativeError::Metadata("no constructed menu is available for the client screen".into())
        })?;
        let inventory = crate::game_object::client_value("client/inventory".to_owned());
        let screen = construct(self, host, constructor, menu, inventory, title)?;
        let layout = host.client_screen_layout(&screen).ok_or_else(|| {
            NativeError::Metadata("screen constructor did not initialize a container screen".into())
        })?;
        Ok(ClientScreenLayout { layout })
    }

    pub fn draw_client_screen_bg(
        &self,
        host: &mut StdHost,
        menu_type_id: Value,
        title: Value,
    ) -> Result<Vec<DrawOp>, NativeError> {
        let constructor = host.client_menu_constructor(&menu_type_id).ok_or_else(|| {
            NativeError::Metadata("no MenuScreens registration for menu type".into())
        })?;
        let menu = host.client_menu_value().ok_or_else(|| {
            NativeError::Metadata("no constructed menu is available for the client screen".into())
        })?;
        let inventory = crate::game_object::client_value("client/inventory".to_owned());
        let screen = construct(self, host, constructor, menu, inventory, title)?;
        if !host.client_frame_active() {
            return Err(NativeError::Metadata("client frame is not active".into()));
        }
        host.client_apply_screen_init(&screen);
        let before = host.client_draws().len();
        let class = crate::expect_ref(&screen)
            .and_then(|key| key.split_once('#').map(|(class, _)| class.to_owned()))
            .ok_or_else(|| NativeError::Metadata("screen is not a jar object".into()))?;
        let rendered = self.invoke_method_with_args(
            &class,
            "extractBackground",
            "(Lnet/minecraft/client/gui/GuiGraphicsExtractor;IIF)V",
            Some(screen.clone()),
            &[
                crate::game_object::client_value("client/gui-graphics".to_owned()),
                Value::Int(0),
                Value::Int(0),
                Value::Float(0.0),
            ],
            host,
        );
        if matches!(rendered, Err(NativeError::MissingMethod(_))) {
            host.client_default_screen_bg(&screen)?;
            let layout = host
                .client_screen_layout(&screen)
                .ok_or_else(|| NativeError::Metadata("screen layout is unavailable".into()))?;
            let _ = self.invoke_method_with_args(
                &class,
                "extractErrorIcon",
                "(Lnet/minecraft/client/gui/GuiGraphicsExtractor;II)V",
                Some(screen),
                &[
                    crate::game_object::client_value("client/gui-graphics".to_owned()),
                    Value::Int(layout.left_pos),
                    Value::Int(layout.top_pos),
                ],
                host,
            )?;
        } else {
            rendered?;
        }
        Ok(host.client_draws()[before..].to_vec())
    }
}
