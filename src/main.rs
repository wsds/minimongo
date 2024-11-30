use ::minimongo::mmg_server::http_server::start_mmg_server;

mod minimongo;
mod mmg_server;

fn main() {
    println!("mmg_server is starting!");
    println!("准备开启mmg_server");
    let _ = start_mmg_server();
    println!("完成开启mmg_server");
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