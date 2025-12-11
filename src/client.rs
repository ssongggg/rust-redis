//! Redis客户端 - 展示Rust的异步IO和用户交互
//!
//! Rust特点展示:
//! - 异步网络IO
//! - 字符串处理
//! - 错误处理

use bytes::BytesMut;
use redis_lib::resp::{RespParser, RespValue};
use redis_lib::DEFAULT_PORT;
use std::env;
use std::io::{self, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析命令行参数
    let (host, port) = parse_args();
    let addr = format!("{}:{}", host, port);

    println!("连接到 {}...", addr);

    // 连接服务器
    let mut stream = TcpStream::connect(&addr).await?;
    println!("已连接！输入 QUIT 退出。\n");

    // 创建读取缓冲区
    let mut buffer = BytesMut::with_capacity(4096);

    // REPL循环
    loop {
        // 显示提示符
        print!("{}:{}> ", host, port);
        io::stdout().flush()?;

        // 读取用户输入
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        // 解析用户输入为RESP命令
        let command = parse_input(input);
        let data = command.serialize();

        // 发送命令
        stream.write_all(&data).await?;

        // 读取响应
        loop {
            let n = stream.read_buf(&mut buffer).await?;
            if n == 0 {
                println!("服务器断开连接");
                return Ok(());
            }

            // 尝试解析响应
            match RespParser::parse(&mut buffer) {
                Ok(Some(response)) => {
                    print_response(&response);
                    break;
                }
                Ok(None) => {
                    // 数据不完整，继续读取
                    continue;
                }
                Err(e) => {
                    eprintln!("解析错误: {}", e);
                    break;
                }
            }
        }

        // 检查是否是QUIT命令
        if input.to_uppercase() == "QUIT" {
            println!("再见！");
            break;
        }
    }

    Ok(())
}

/// 解析命令行参数
fn parse_args() -> (String, u16) {
    let args: Vec<String> = env::args().collect();

    let host = args.get(1).cloned().unwrap_or_else(|| "127.0.0.1".to_string());

    let port = args
        .get(2)
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    (host, port)
}

/// 将用户输入解析为RESP数组
///
/// Rust特点: 迭代器和闭包的组合
fn parse_input(input: &str) -> RespValue {
    // 简单的空格分割，支持引号内的空格
    let parts = tokenize(input);

    RespValue::Array(
        parts
            .into_iter()
            .map(|s| RespValue::BulkString(s.into_bytes()))
            .collect(),
    )
}

/// 分词器 - 支持引号
///
/// Rust特点: 状态机模式匹配
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = '"';

    for c in input.chars() {
        match c {
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = c;
            }
            c if c == quote_char && in_quotes => {
                in_quotes = false;
            }
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

/// 格式化打印响应
///
/// Rust特点: 递归模式匹配
fn print_response(value: &RespValue) {
    print_response_inner(value, 0);
}

fn print_response_inner(value: &RespValue, indent: usize) {
    let prefix = "  ".repeat(indent);

    match value {
        RespValue::SimpleString(s) => {
            println!("{}\"{s}\"", prefix);
        }
        RespValue::Error(e) => {
            println!("{}(error) {}", prefix, e);
        }
        RespValue::Integer(i) => {
            println!("{}(integer) {}", prefix, i);
        }
        RespValue::BulkString(data) => {
            match String::from_utf8(data.clone()) {
                Ok(s) => println!("{}\"{s}\"", prefix),
                Err(_) => println!("{}<binary data, {} bytes>", prefix, data.len()),
            }
        }
        RespValue::Null => {
            println!("{}(nil)", prefix);
        }
        RespValue::Array(arr) => {
            if arr.is_empty() {
                println!("{}(empty array)", prefix);
            } else {
                for (i, item) in arr.iter().enumerate() {
                    print!("{}{}) ", prefix, i + 1);
                    // 数组元素不需要额外缩进前缀
                    match item {
                        RespValue::SimpleString(s) => println!("\"{s}\""),
                        RespValue::Error(e) => println!("(error) {}", e),
                        RespValue::Integer(i) => println!("(integer) {}", i),
                        RespValue::BulkString(data) => {
                            match String::from_utf8(data.clone()) {
                                Ok(s) => println!("\"{s}\""),
                                Err(_) => println!("<binary data, {} bytes>", data.len()),
                            }
                        }
                        RespValue::Null => println!("(nil)"),
                        RespValue::Array(_) => {
                            println!();
                            print_response_inner(item, indent + 1);
                        }
                    }
                }
            }
        }
    }
}

