pub mod app;
pub mod catalog;
pub mod clean;
pub mod reveal;
pub mod scan;
pub mod window;

pub mod events {
    pub const TARGET: &str = "scan://target";
    pub const STATUS: &str = "scan://status";
}
