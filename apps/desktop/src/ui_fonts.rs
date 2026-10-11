use gpui::App;

/// CDXC:DesignSystem 2026-09-26 DECISION:
/// User: Windows should render with the same quality, type sizes and spacing as macOS.
pub(crate) const UI_FONT: &str = "Inter";

/// CDXC:DesignSystem 2026-09-26 WHY:
/// The bundled static faces identify as Inter, not Inter Variable. Register them before the first window so chrome and Docs use the same metrics without relying on fonts installed on the host.
pub(crate) fn register(cx: &App) {
    static REGISTERED: std::sync::Once = std::sync::Once::new();
    REGISTERED.call_once(|| {
        let fonts = vec![
            include_bytes!("../../../.dependencies/app-fonts/inter/Inter-Light.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../../.dependencies/app-fonts/inter/Inter-Regular.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../../.dependencies/app-fonts/inter/Inter-Medium.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../../.dependencies/app-fonts/inter/Inter-SemiBold.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../../.dependencies/app-fonts/inter/Inter-Bold.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../../.dependencies/app-fonts/inter/Inter-Italic.ttf")
                .as_slice()
                .into(),
        ];
        if let Err(error) = cx.text_system().add_fonts(fonts) {
            eprintln!("[ghostex-gpui] Could not register UI fonts: {error}");
        }
    });
}
