//! src/common.rs
/// 一个简单的计数器，用于在整个编译流程中生成唯一的标识符。
pub struct UniqueIdGenerator {
    counter: usize,
}
impl UniqueIdGenerator {
    pub fn new() -> Self {
        UniqueIdGenerator { counter: 0 }
    }
    /// 获取下一个唯一的数字标识符。
    pub fn next(&mut self) -> usize {
        let id = self.counter;
        self.counter += 1;
        id
    }
}
