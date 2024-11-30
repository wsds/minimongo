mod minimongo;
mod mmg_server;

fn main() {
    println!("Hello, world!");
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