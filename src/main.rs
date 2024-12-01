use crate::common::helper::empty_loop;
use crate::mmg_server::http_server::start_mmg_server_sub_thread;

mod minimongo;
mod mmg_server;
pub mod common;

fn main() {
    println!("mmg_server is starting!");
    println!("准备开启mmg_server 100001");
    let _ = start_mmg_server_sub_thread();
    println!("完成开启mmg_server");
    empty_loop();
}


#[cfg(test)]
mod tests {

    //cargo test test_mini_mongo_main -- --nocapture
    #[test]
    fn test_mini_mongo_main() -> Result<(), Box<i32>> {
        println!("准备测试: test_mini_mongo_main");
        println!("测试完成: test_mini_mongo_main");
        Ok(())
    }
}