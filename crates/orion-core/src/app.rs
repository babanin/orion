#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    None,
    RedrawFull,
    ExitToLauncher,
}
