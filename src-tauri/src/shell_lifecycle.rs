#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowRestoreStep {
    ActivateApp,
    Create,
    Unminimize,
    Reposition,
    Show,
    Focus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowLifecycleState {
    pub visible: bool,
    pub minimized: bool,
    pub on_screen: bool,
    pub focused: bool,
}

impl WindowLifecycleState {
    pub fn restore_steps(self) -> Vec<WindowRestoreStep> {
        let mut steps = vec![WindowRestoreStep::ActivateApp];
        if self.minimized {
            steps.push(WindowRestoreStep::Unminimize);
        }
        steps.extend([
            WindowRestoreStep::Reposition,
            WindowRestoreStep::Show,
            WindowRestoreStep::Focus,
        ]);
        steps
    }
}
