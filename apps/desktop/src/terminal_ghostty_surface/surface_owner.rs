use std::{
    ffi::c_void,
    mem::ManuallyDrop,
    ptr::{self, NonNull},
};

use gpui::{Bounds, Pixels};

use crate::{
    AgentsTerminalBodyMountSlotId, AgentsTerminalRuntimeSessionId, AgentsTerminalStartupBodySlotId,
    TerminalSurfaceMountSlotKey, ghostty_kit::ffi,
};

use super::*;

pub(crate) struct GhosttySurfaceOwner<SlotId = AgentsTerminalBodyMountSlotId> {
    surface: NonNull<c_void>,
    mount_slot_id: SlotId,
    runtime_session_id: AgentsTerminalRuntimeSessionId,
    pub(crate) functions: GhosttyKitFunctionTable,
    close_token: Box<GhosttySurfaceCloseToken>,
    close_requested: bool,
    latest_scale_factor: Option<GhosttySurfaceScaleFactor>,
    latest_pixel_size: Option<GhosttySurfacePixelSize>,
    latest_focus_state: Option<bool>,
    latest_occlusion_state: bool,
}

impl<SlotId> GhosttySurfaceOwner<SlotId>
where
    SlotId: TerminalSurfaceMountSlotKey,
{
    pub(crate) fn new(
        app: &GhosttyAppOwner,
        mount_slot_id: SlotId,
        runtime_session_id: AgentsTerminalRuntimeSessionId,
        request: &GhosttySurfaceConfigRequest,
    ) -> Result<Self, GhosttySurfaceRuntimeError> {
        let close_token = GhosttySurfaceCloseToken::boxed(app.functions);
        let (surface, functions) =
            create_ghostty_surface_from_request(app, request, close_token.as_userdata())?;
        close_token.set_surface(surface.as_ptr());
        Ok(Self {
            surface,
            mount_slot_id,
            runtime_session_id,
            functions,
            close_token,
            close_requested: false,
            latest_scale_factor: None,
            latest_pixel_size: None,
            latest_focus_state: None,
            latest_occlusion_state: true,
        })
    }

    pub(crate) fn mount_slot_id(&self) -> SlotId {
        self.mount_slot_id
    }

    pub(crate) fn runtime_session_id(&self) -> AgentsTerminalRuntimeSessionId {
        self.runtime_session_id
    }

    pub(crate) fn can_rekey_to_mount_slot(
        &self,
        mount_slot_id: SlotId,
        runtime_session_id: AgentsTerminalRuntimeSessionId,
    ) -> bool {
        self.mount_slot_id == mount_slot_id && self.runtime_session_id == runtime_session_id
    }

    pub(crate) fn can_move_to_mount_slot(
        &self,
        runtime_session_id: AgentsTerminalRuntimeSessionId,
    ) -> bool {
        self.runtime_session_id == runtime_session_id
    }

    pub(crate) fn into_rekeyed_surface_owner(
        self,
        mount_slot_id: SlotId,
        runtime_session_id: AgentsTerminalRuntimeSessionId,
    ) -> Self {
        /*
        CDXC:Terminal 2026-06-23-19:41:
        Parked Running owners must reattach by moving the same Ghostty surface back under the current body slot. `ManuallyDrop` prevents `ghostty_surface_free` during the rekey and keeps the close/clipboard token with the surface userdata so reattach cannot recreate the process or lose owner-scoped runtime callbacks.
        */
        let owner = ManuallyDrop::new(self);
        let close_token = unsafe { ptr::read(&owner.close_token) };
        Self {
            surface: owner.surface,
            mount_slot_id,
            runtime_session_id,
            functions: owner.functions,
            close_token,
            close_requested: owner.close_requested,
            latest_scale_factor: owner.latest_scale_factor,
            latest_pixel_size: owner.latest_pixel_size,
            latest_focus_state: None,
            latest_occlusion_state: owner.latest_occlusion_state,
        }
    }

    pub(crate) fn update_content_scale_and_size(
        &mut self,
        bounds: Bounds<Pixels>,
        scale_factor: f64,
    ) -> Result<(), GhosttySurfaceRuntimeError> {
        let scale_factor = GhosttySurfaceScaleFactor::new(scale_factor)?;
        let pixel_size = GhosttySurfacePixelSize::from_gpui_bounds(bounds, scale_factor.get())?;
        self.set_content_scale(scale_factor);
        self.set_size_pixels(pixel_size);
        Ok(())
    }

    pub(crate) fn set_focus(&mut self, focused: bool) {
        if self.latest_focus_state == Some(focused) {
            return;
        }
        unsafe {
            (self.functions.surface_set_focus)(self.as_raw(), focused);
        }
        self.latest_focus_state = Some(focused);
    }

    pub(crate) fn set_occlusion(&mut self, visible: bool) {
        if self.latest_occlusion_state == visible {
            return;
        }
        unsafe {
            (self.functions.surface_set_occlusion)(self.as_raw(), visible);
        }
        self.latest_occlusion_state = visible;
    }

    pub(crate) fn surface_size(&self) -> ffi::ghostty_surface_size_s {
        unsafe { (self.functions.surface_size)(self.as_raw()) }
    }

    #[allow(dead_code)] // no live caller: the native ghostty surface host owns this today
    pub(crate) fn metadata_snapshot(&self) -> GhosttySurfaceMetadataSnapshot {
        ghostty_surface_metadata_snapshot(self.functions, self.as_raw())
    }

    pub(crate) fn process_exited(&self) -> bool {
        unsafe { (self.functions.surface_process_exited)(self.as_raw()) }
    }

    pub(crate) fn needs_confirm_quit(&self) -> bool {
        unsafe { (self.functions.surface_needs_confirm_quit)(self.as_raw()) }
    }

    /// Performs a named Ghostty keybind action (e.g. `start_search`,
    /// `search:<needle>`, `navigate_search:next`, `end_search`) on this
    /// surface, mirroring the macOS host's `performBindingAction`.
    pub(crate) fn perform_binding_action(&self, action: &str) -> bool {
        unsafe {
            (self.functions.surface_binding_action)(
                self.as_raw(),
                action.as_ptr().cast(),
                action.len(),
            )
        }
    }

    #[allow(dead_code)] // no live caller: the native ghostty surface host owns this today
    pub(crate) fn key_translation_mods(
        &self,
        mods: ffi::ghostty_input_mods_e,
    ) -> ffi::ghostty_input_mods_e {
        unsafe { (self.functions.surface_key_translation_mods)(self.as_raw(), mods) }
    }

    pub(crate) fn send_key(&self, event: ffi::ghostty_input_key_s) -> bool {
        unsafe { (self.functions.surface_key)(self.as_raw(), event) }
    }

    #[allow(dead_code)] // no live caller: the native ghostty surface host owns this today
    pub(crate) fn key_is_binding(
        &self,
        event: ffi::ghostty_input_key_s,
    ) -> GhosttySurfaceKeyBindingStatus {
        let mut flags = 0;
        let binding =
            unsafe { (self.functions.surface_key_is_binding)(self.as_raw(), event, &mut flags) };
        GhosttySurfaceKeyBindingStatus::from_ffi_result(binding, flags)
    }

    pub(crate) fn send_text_bytes(&self, bytes: &[u8]) {
        unsafe {
            (self.functions.surface_text)(
                self.as_raw(),
                ghostty_surface_text_ptr(bytes),
                bytes.len(),
            );
        }
    }

    pub(crate) fn set_preedit_bytes(&self, bytes: &[u8]) {
        unsafe {
            (self.functions.surface_preedit)(
                self.as_raw(),
                ghostty_surface_preedit_ptr(bytes),
                bytes.len(),
            );
        }
    }

    pub(crate) fn mouse_captured(&self) -> bool {
        unsafe { (self.functions.surface_mouse_captured)(self.as_raw()) }
    }

    pub(crate) fn mouse_button(
        &self,
        action: ffi::ghostty_input_mouse_state_e,
        button: ffi::ghostty_input_mouse_button_e,
        mods: ffi::ghostty_input_mods_e,
    ) -> bool {
        unsafe { (self.functions.surface_mouse_button)(self.as_raw(), action, button, mods) }
    }

    pub(crate) fn mouse_pos(&self, x: f64, y: f64, mods: ffi::ghostty_input_mods_e) {
        unsafe {
            (self.functions.surface_mouse_pos)(self.as_raw(), x, y, mods);
        }
    }

    pub(crate) fn mouse_scroll(
        &self,
        x: f64,
        y: f64,
        scroll_mods: ffi::ghostty_input_scroll_mods_t,
    ) {
        unsafe {
            (self.functions.surface_mouse_scroll)(self.as_raw(), x, y, scroll_mods);
        }
    }

    pub(crate) fn mouse_pressure(&self, stage: u32, pressure: f64) {
        unsafe {
            (self.functions.surface_mouse_pressure)(self.as_raw(), stage, pressure);
        }
    }

    pub(crate) fn ime_point(&self) -> GhosttySurfaceImePoint {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut width = 0.0;
        let mut height = 0.0;
        unsafe {
            (self.functions.surface_ime_point)(
                self.as_raw(),
                &mut x,
                &mut y,
                &mut width,
                &mut height,
            );
        }
        GhosttySurfaceImePoint {
            x,
            y,
            width,
            height,
        }
    }

    pub(crate) fn request_close(&mut self) -> bool {
        if self.close_requested {
            return false;
        }
        self.close_requested = true;
        unsafe {
            (self.functions.surface_request_close)(self.as_raw());
        }
        true
    }

    pub(crate) fn consume_confirmed_close_requested(&self) -> bool {
        self.close_token.consume_confirmed_close_requested()
    }

    pub(crate) fn consume_confirmation_needed_close_requested(&self) -> bool {
        self.close_token
            .consume_confirmation_needed_close_requested()
    }

    pub(crate) fn cancel_pending_close_request(&mut self) -> bool {
        if !self.close_requested || self.close_token.confirmed_close_pending() {
            return false;
        }
        self.close_token.clear_confirmation_needed_close_requested();
        self.close_requested = false;
        true
    }

    pub(crate) fn drain_runtime_clipboard_requests(
        &self,
        allow_standard_clipboard: bool,
        read_standard_text: impl FnMut() -> Option<String>,
        write_standard_text: impl FnMut(String),
    ) {
        self.close_token.drain_runtime_clipboard_operations(
            allow_standard_clipboard,
            read_standard_text,
            write_standard_text,
        );
    }

    pub(crate) fn drain_runtime_action_events(&self) -> Vec<GhosttyRuntimeActionEvent> {
        self.close_token
            .take_runtime_action_events()
            .into_iter()
            .collect()
    }

    pub(crate) fn as_raw(&self) -> ffi::ghostty_surface_t {
        self.surface.as_ptr()
    }

    fn set_content_scale(&mut self, scale_factor: GhosttySurfaceScaleFactor) {
        if self.latest_scale_factor == Some(scale_factor) {
            return;
        }
        unsafe {
            (self.functions.surface_set_content_scale)(
                self.as_raw(),
                scale_factor.get(),
                scale_factor.get(),
            );
        }
        self.latest_scale_factor = Some(scale_factor);
    }

    fn set_size_pixels(&mut self, pixel_size: GhosttySurfacePixelSize) {
        if self.latest_pixel_size == Some(pixel_size) {
            return;
        }
        unsafe {
            (self.functions.surface_set_size)(self.as_raw(), pixel_size.width, pixel_size.height);
        }
        self.latest_pixel_size = Some(pixel_size);
    }
}

impl<SlotId> Drop for GhosttySurfaceOwner<SlotId> {
    fn drop(&mut self) {
        self.close_token.deny_pending_runtime_clipboard_operations();
        unsafe {
            (self.functions.surface_free)(self.surface.as_ptr());
        }
        self.close_token.clear_surface();
    }
}

pub(crate) struct StartupGhosttySurfaceOwner {
    surface: NonNull<c_void>,
    startup_body_slot_id: AgentsTerminalStartupBodySlotId,
    runtime_session_id: AgentsTerminalRuntimeSessionId,
    functions: GhosttyKitFunctionTable,
    close_token: Box<GhosttySurfaceCloseToken>,
    latest_scale_factor: Option<GhosttySurfaceScaleFactor>,
    latest_pixel_size: Option<GhosttySurfacePixelSize>,
}

impl StartupGhosttySurfaceOwner {
    pub(crate) fn new(
        app: &GhosttyAppOwner,
        startup_body_slot_id: AgentsTerminalStartupBodySlotId,
        runtime_session_id: AgentsTerminalRuntimeSessionId,
        request: &GhosttySurfaceConfigRequest,
    ) -> Result<Self, GhosttySurfaceRuntimeError> {
        let close_token = GhosttySurfaceCloseToken::boxed(app.functions);
        let (surface, functions) =
            create_ghostty_surface_from_request(app, request, close_token.as_userdata())?;
        close_token.set_surface(surface.as_ptr());
        Ok(Self {
            surface,
            startup_body_slot_id,
            runtime_session_id,
            functions,
            close_token,
            latest_scale_factor: None,
            latest_pixel_size: None,
        })
    }

    pub(crate) fn startup_body_slot_id(&self) -> AgentsTerminalStartupBodySlotId {
        self.startup_body_slot_id
    }

    pub(crate) fn runtime_session_id(&self) -> AgentsTerminalRuntimeSessionId {
        self.runtime_session_id
    }

    pub(crate) fn update_content_scale_and_size(
        &mut self,
        bounds: Bounds<Pixels>,
        scale_factor: f64,
    ) -> Result<(), GhosttySurfaceRuntimeError> {
        let scale_factor = GhosttySurfaceScaleFactor::new(scale_factor)?;
        let pixel_size = GhosttySurfacePixelSize::from_gpui_bounds(bounds, scale_factor.get())?;
        self.set_content_scale(scale_factor);
        self.set_size_pixels(pixel_size);
        Ok(())
    }

    pub(crate) fn metadata_snapshot(&self) -> GhosttySurfaceMetadataSnapshot {
        ghostty_surface_metadata_snapshot(self.functions, self.as_raw())
    }

    pub(crate) fn into_running_surface_owner(
        self,
        mount_slot_id: AgentsTerminalBodyMountSlotId,
    ) -> GhosttySurfaceOwner {
        /*
        CDXC:Terminal 2026-06-23-04:25:
        Promotion must transfer the exact startup Ghostty surface into the Running owner without calling `ghostty_surface_free`. `ManuallyDrop` keeps the surface alive while the new owner takes the same raw handle and runtime id; focus starts unset because startup owners never focus hidden hosts.

        CDXC:Terminal 2026-06-23-04:49:
        The surface userdata is the owner-held close token, so Ready handoff must move that token with the raw Ghostty surface. Replacing it would leave the embedded close callback pointing at stale process memory.

        CDXC:Clipboard 2026-06-27-04:10:
        The surface userdata carries the registered close/clipboard token, so Ready handoff must move that token with the Ghostty surface. Clipboard callbacks can then keep enqueueing owner-local operations for the promoted Running owner without recreating the process, using focus, or falling back to app-level runtime userdata.
        */
        let startup_owner = ManuallyDrop::new(self);
        let close_token = unsafe { ptr::read(&startup_owner.close_token) };
        GhosttySurfaceOwner {
            surface: startup_owner.surface,
            mount_slot_id,
            runtime_session_id: startup_owner.runtime_session_id,
            functions: startup_owner.functions,
            close_token,
            close_requested: false,
            latest_scale_factor: startup_owner.latest_scale_factor,
            latest_pixel_size: startup_owner.latest_pixel_size,
            latest_focus_state: None,
            latest_occlusion_state: true,
        }
    }

    pub(crate) fn as_raw(&self) -> ffi::ghostty_surface_t {
        self.surface.as_ptr()
    }

    fn set_content_scale(&mut self, scale_factor: GhosttySurfaceScaleFactor) {
        if self.latest_scale_factor == Some(scale_factor) {
            return;
        }
        unsafe {
            (self.functions.surface_set_content_scale)(
                self.as_raw(),
                scale_factor.get(),
                scale_factor.get(),
            );
        }
        self.latest_scale_factor = Some(scale_factor);
    }

    fn set_size_pixels(&mut self, pixel_size: GhosttySurfacePixelSize) {
        if self.latest_pixel_size == Some(pixel_size) {
            return;
        }
        unsafe {
            (self.functions.surface_set_size)(self.as_raw(), pixel_size.width, pixel_size.height);
        }
        self.latest_pixel_size = Some(pixel_size);
    }
}

impl Drop for StartupGhosttySurfaceOwner {
    fn drop(&mut self) {
        self.close_token.deny_pending_runtime_clipboard_operations();
        unsafe {
            (self.functions.surface_free)(self.as_raw());
        }
        self.close_token.clear_surface();
    }
}

fn create_ghostty_surface_from_request(
    app: &GhosttyAppOwner,
    request: &GhosttySurfaceConfigRequest,
    surface_userdata: *mut c_void,
) -> Result<(NonNull<c_void>, GhosttyKitFunctionTable), GhosttySurfaceRuntimeError> {
    let functions = app.functions;
    let config = unsafe { (functions.surface_config_new)() };
    let mut prepared_config = request.prepare_ffi_config(config);
    prepared_config.set_surface_userdata(surface_userdata);
    let surface = unsafe { (functions.surface_new)(app.as_raw(), prepared_config.as_ptr()) };
    let surface =
        NonNull::new(surface).ok_or(GhosttySurfaceRuntimeError::SurfaceCreateReturnedNull)?;
    Ok((surface, functions))
}
