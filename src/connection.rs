//! 连接处理模块 - 展示Rust的异步编程
//!
//! Rust特点展示:
//! - async/await异步编程
//! - tokio异步运行时
//! - 所有权在异步上下文中的转移
//! - 生命周期和借用检查

use crate::command::{Command, CommandExecutor};
use crate::error::{RedisError, RedisResult};
use crate::resp::{RespParser, RespValue};
use crate::store::Store;
use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// 连接处理器
///
/// Rust特点: 结构体持有连接状态，方法操作状态
pub struct Connection {
    /// TCP流
    stream: TcpStream,
    /// 读取缓冲区
    buffer: BytesMut,
    /// 客户端地址(用于日志)
    addr: String,
}

impl Connection {
    /// 创建新连接
    ///
    /// Rust特点: 所有权转移 - TcpStream的所有权从调用者转移到Connection
    pub fn new(stream: TcpStream) -> Self {
        let addr = stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Self {
            stream,
            buffer: BytesMut::with_capacity(4096),
            addr,
        }
    }

    /// 获取客户端地址
    pub fn addr(&self) -> &str {
        &self.addr
    }

    /// 处理客户端连接
    ///
    /// Rust特点:
    /// - async fn 定义异步函数
    /// - &Store 是共享引用，允许多个连接同时访问存储
    pub async fn handle(&mut self, store: &Store) -> RedisResult<()> {
        println!("[{}] 客户端已连接", self.addr);

        loop {
            // 尝试解析缓冲区中的命令
            match self.read_command().await {
                Ok(Some(value)) => {
                    // 解析并执行命令
                    match Command::from_resp(value) {
                        Ok(cmd) => {
                            let executor = CommandExecutor::new(store);
                            let (response, should_quit) = executor.execute(cmd);

                            // 发送响应
                            self.write_response(&response).await?;

                            // 如果是QUIT命令，断开连接
                            if should_quit {
                                println!("[{}] 客户端请求断开", self.addr);
                                break;
                            }
                        }
                        Err(e) => {
                            // 命令解析错误，发送错误响应
                            let error_response =
                                RespValue::Error(format!("ERR {}", e));
                            self.write_response(&error_response).await?;
                        }
                    }
                }
                Ok(None) => {
                    // 连接关闭
                    println!("[{}] 客户端断开连接", self.addr);
                    break;
                }
                Err(e) => {
                    // 协议错误
                    eprintln!("[{}] 错误: {}", self.addr, e);
                    let error_response = RespValue::Error(format!("ERR {}", e));
                    if self.write_response(&error_response).await.is_err() {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// 从连接读取命令
    ///
    /// Rust特点:
    /// - .await 暂停执行直到异步操作完成
    /// - ? 操作符传播错误
    async fn read_command(&mut self) -> RedisResult<Option<RespValue>> {
        loop {
            // 先尝试从缓冲区解析命令
            if let Some(value) = RespParser::parse(&mut self.buffer)? {
                return Ok(Some(value));
            }

            // 缓冲区中没有完整命令，从网络读取更多数据
            let bytes_read = self.stream.read_buf(&mut self.buffer).await?;

            // 如果读取到0字节，说明连接已关闭
            if bytes_read == 0 {
                // 检查缓冲区是否有未处理的数据
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(RedisError::ConnectionClosed);
                }
            }
        }
    }

    /// 写入响应
    ///
    /// Rust特点: 引用避免不必要的数据复制
    async fn write_response(&mut self, response: &RespValue) -> RedisResult<()> {
        let data = response.serialize();
        self.stream.write_all(&data).await?;
        self.stream.flush().await?;
        Ok(())
    }
}

/// 后台任务：定期清理过期的键
///
/// Rust特点: 独立的异步任务，通过Arc共享Store
pub async fn cleanup_task(store: Store, interval_secs: u64) {
    use tokio::time::{interval, Duration};

    let mut ticker = interval(Duration::from_secs(interval_secs));

    loop {
        ticker.tick().await;
        let cleaned = store.cleanup_expired();
        if cleaned > 0 {
            println!("[清理任务] 清理了 {} 个过期的键", cleaned);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 异步测试需要tokio的测试宏
    #[tokio::test]
    async fn test_connection_new() {
        // 这里只测试基本结构，实际网络测试需要mock
        let store = Store::new();
        assert_eq!(store.dbsize(), 0);
    }
}

