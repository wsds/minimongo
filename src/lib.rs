
pub mod minimongo;
pub mod mmg_server;
pub mod common;

pub use mmg_server::http_server::start_mmg_server;
pub use mmg_server::http_server::start_mmg_server_sub_thread;
pub use minimongo::minimongo::get_mgdb;
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(12312, 2);
        assert_eq!(result, 4);
    }
}
