//! Rust Redis - 一个用Rust实现的简单Redis服务器
//!
//! 这个项目展示了Rust语言的多种特性：
//!
//! ## Rust语言特点展示
//!
//! ### 1. 所有权系统 (Ownership)
//! - 每个值都有一个所有者
//! - 同一时刻只能有一个所有者
//! - 当所有者离开作用域，值被自动释放
//!
//! ### 2. 借用和引用 (Borrowing & References)
//! - `&T` 不可变借用
//! - `&mut T` 可变借用
//! - 借用检查器在编译期保证内存安全
//!
//! ### 3. 枚举和模式匹配 (Enum & Pattern Matching)
//! - 代数数据类型
//! - `match` 必须穷尽所有情况
//! - `if let` 和 `while let` 简化匹配
//!
//! ### 4. 错误处理 (Error Handling)
//! - `Result<T, E>` 表示可能失败的操作
//! - `Option<T>` 表示可能为空的值
//! - `?` 操作符简化错误传播
//!
//! ### 5. Trait系统
//! - 类似接口，定义共享行为
//! - 可以为任何类型实现trait
//! - 派生宏自动实现常用trait
//!
//! ### 6. 并发安全
//! - `Arc<T>` 原子引用计数
//! - `Mutex<T>` / `RwLock<T>` 互斥锁/读写锁
//! - 类型系统保证线程安全 (`Send` + `Sync`)
//!
//! ### 7. 异步编程 (Async/Await)
//! - `async fn` 定义异步函数
//! - `.await` 等待异步操作完成
//! - 基于Future的零成本抽象
//!
//! ### 8. 生命周期 (Lifetimes)
//! - `'a` 标注引用的有效范围
//! - 确保引用不会比被引用的数据活得更久
//!
//! ## 模块结构
//!
//! - `error` - 错误处理
//! - `resp` - RESP协议解析
//! - `store` - 数据存储
//! - `command` - 命令处理
//! - `connection` - 连接处理

pub mod command;
pub mod connection;
pub mod error;
pub mod resp;
pub mod store;

// 重新导出常用类型
pub use error::{RedisError, RedisResult};
pub use resp::RespValue;
pub use store::Store;

/// 默认端口
pub const DEFAULT_PORT: u16 = 6379;

/// 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

