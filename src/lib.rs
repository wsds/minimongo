
pub mod minimongo;
pub mod mmg_server;

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
