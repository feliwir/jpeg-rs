mod fixed;
mod float;
use super::tables::{COS_TABLE, COS_TABLE_FIXED};
pub use fixed::idct_fixed;
pub use float::idct;
