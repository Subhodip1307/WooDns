mod fileworker;
mod interface;
pub use fileworker::{LogHandeler, all_write_now, get_file_count};
pub use interface::DnsLogger;
