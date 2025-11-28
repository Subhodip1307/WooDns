mod fileworker;
mod interface;
pub use fileworker::{LogHandler, all_write_now, get_file_count};
pub use interface::DnsLogger;
