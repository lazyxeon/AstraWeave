pub mod menu_bar;
pub mod progress;
pub mod status_bar;
pub mod toast;

pub use menu_bar::{AlignDirection, DistributeDirection, MenuActionHandler, MenuBar};
pub use progress::{ProgressManager, TaskCategory, TaskId};
pub use status_bar::{ResourceUsage, StatusBar};
#[allow(unused_imports)]
pub use toast::{ToastAction, ToastLevel, ToastManager};

#[cfg(test)]
mod tests_progress;
#[cfg(test)]
mod tests_toast;
// Toast struct is internal to ToastManager, not exported directly
// BackgroundTaskSummary available via status_bar module if needed
