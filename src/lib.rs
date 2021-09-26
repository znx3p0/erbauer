
pub use erbauer_proc::{tasks, erbauer};
pub use once_cell::sync::OnceCell;


pub trait Task {
    type Dependencies: Default;
    fn __run(task: Self::Dependencies) -> &'static Self;
    fn run() -> &'static Self {
        Self::__run(Default::default())
    }
}
