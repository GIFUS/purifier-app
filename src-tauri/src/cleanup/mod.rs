pub mod catalog;
pub mod categories;
pub mod clean;
pub mod fs;
pub mod locate;
pub mod model;
pub mod report;
pub mod scan;
pub mod validate;

pub mod events {
    pub const TARGET: &str = "scan://target";
    pub const STATUS: &str = "scan://status";
}
