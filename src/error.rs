//! 错误处理模块 - 展示Rust的错误处理机制
//!
//! Rust特点展示:
//! - 使用thiserror派生宏简化错误定义
//! - 枚举类型表示不同错误种类
//! - Result类型进行错误传播

use std::io;
use std::num::ParseIntError;
use std::string::FromUtf8Error;
use thiserror::Error;

/// Redis错误类型 - 使用枚举统一管理所有可能的错误
///
/// Rust特点: 枚举可以携带数据，配合thiserror可以自动实现Error trait
#[derive(Debug, Error)]
pub enum RedisError {
    /// IO错误 - 网络或文件操作失败
    #[error("IO错误: {0}")]
    Io(#[from] io::Error),

    /// 协议解析错误
    #[error("协议错误: {0}")]
    Protocol(String),

    /// 无效的命令
    #[error("未知命令: {0}")]
    UnknownCommand(String),

    /// 参数数量错误
    #[error("参数数量错误: 命令 '{command}' 需要 {expected} 个参数，但收到 {got} 个")]
    WrongNumberOfArguments {
        command: String,
        expected: usize,
        got: usize,
    },

    /// 类型错误
    #[error("类型错误: {0}")]
    TypeError(String),

    /// UTF-8解析错误
    #[error("UTF-8解析错误: {0}")]
    Utf8Error(#[from] FromUtf8Error),

    /// 整数解析错误
    #[error("整数解析错误: {0}")]
    ParseIntError(#[from] ParseIntError),

    /// 连接已关闭
    #[error("连接已关闭")]
    ConnectionClosed,

    /// 内部错误
    #[error("内部错误: {0}")]
    Internal(String),
}

/// 自定义Result类型别名 - 简化代码
///
/// Rust特点: 类型别名提高代码可读性
pub type RedisResult<T> = Result<T, RedisError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = RedisError::UnknownCommand("INVALID".to_string());
        assert!(err.to_string().contains("INVALID"));

        let err = RedisError::WrongNumberOfArguments {
            command: "SET".to_string(),
            expected: 2,
            got: 1,
        };
        assert!(err.to_string().contains("SET"));
    }
}

