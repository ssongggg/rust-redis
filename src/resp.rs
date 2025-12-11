//! RESP协议解析模块 - 展示Rust的模式匹配和枚举
//!
//! RESP (REdis Serialization Protocol) 是Redis的通信协议
//!
//! Rust特点展示:
//! - 枚举类型表示不同数据类型
//! - 模式匹配处理不同情况
//! - 所有权和借用
//! - 递归数据结构

use crate::error::{RedisError, RedisResult};
use bytes::{Buf, BytesMut};

/// RESP数据类型 - 使用枚举表示协议中的不同数据类型
///
/// Rust特点: 枚举可以携带不同类型的数据(代数数据类型)
#[derive(Debug, Clone, PartialEq)]
pub enum RespValue {
    /// 简单字符串: +OK\r\n
    SimpleString(String),
    /// 错误: -Error message\r\n
    Error(String),
    /// 整数: :1000\r\n
    Integer(i64),
    /// 批量字符串: $6\r\nfoobar\r\n
    BulkString(Vec<u8>),
    /// 空值: $-1\r\n
    Null,
    /// 数组: *2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n
    Array(Vec<RespValue>),
}

impl RespValue {
    /// 将RESP值序列化为字节
    ///
    /// Rust特点: match表达式必须穷尽所有情况，编译器保证完整性
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            // 简单字符串
            RespValue::SimpleString(s) => format!("+{}\r\n", s).into_bytes(),

            // 错误
            RespValue::Error(e) => format!("-{}\r\n", e).into_bytes(),

            // 整数
            RespValue::Integer(i) => format!(":{}\r\n", i).into_bytes(),

            // 批量字符串
            RespValue::BulkString(data) => {
                let mut result = format!("${}\r\n", data.len()).into_bytes();
                result.extend_from_slice(data);
                result.extend_from_slice(b"\r\n");
                result
            }

            // 空值
            RespValue::Null => b"$-1\r\n".to_vec(),

            // 数组 - 递归序列化
            RespValue::Array(arr) => {
                let mut result = format!("*{}\r\n", arr.len()).into_bytes();
                for item in arr {
                    result.extend(item.serialize());
                }
                result
            }
        }
    }

    /// 尝试将RESP值转换为字符串
    ///
    /// Rust特点: Option类型表示可能为空的值，避免空指针
    pub fn as_string(&self) -> Option<String> {
        match self {
            RespValue::SimpleString(s) => Some(s.clone()),
            RespValue::BulkString(data) => String::from_utf8(data.clone()).ok(),
            _ => None,
        }
    }

    /// 尝试将RESP值转换为整数
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            RespValue::Integer(i) => Some(*i),
            RespValue::BulkString(data) => {
                String::from_utf8(data.clone())
                    .ok()
                    .and_then(|s| s.parse().ok())
            }
            _ => None,
        }
    }

    /// 判断是否为空值
    pub fn is_null(&self) -> bool {
        matches!(self, RespValue::Null)
    }
}

/// RESP解析器
///
/// Rust特点: 结构体封装状态，方法操作状态
pub struct RespParser;

impl RespParser {
    /// 从缓冲区解析RESP值
    ///
    /// Rust特点:
    /// - &mut BytesMut 是可变借用，允许修改缓冲区
    /// - Result类型处理可能的错误
    pub fn parse(buf: &mut BytesMut) -> RedisResult<Option<RespValue>> {
        if buf.is_empty() {
            return Ok(None);
        }

        // 检查是否有完整的行
        let first_byte = buf[0];

        match first_byte {
            b'+' => Self::parse_simple_string(buf),
            b'-' => Self::parse_error(buf),
            b':' => Self::parse_integer(buf),
            b'$' => Self::parse_bulk_string(buf),
            b'*' => Self::parse_array(buf),
            // 处理内联命令(如 PING)
            _ => Self::parse_inline_command(buf),
        }
    }

    /// 解析简单字符串
    fn parse_simple_string(buf: &mut BytesMut) -> RedisResult<Option<RespValue>> {
        if let Some(line) = Self::read_line(buf)? {
            // 跳过 '+' 前缀
            let content = String::from_utf8(line[1..].to_vec())?;
            Ok(Some(RespValue::SimpleString(content)))
        } else {
            Ok(None)
        }
    }

    /// 解析错误
    fn parse_error(buf: &mut BytesMut) -> RedisResult<Option<RespValue>> {
        if let Some(line) = Self::read_line(buf)? {
            let content = String::from_utf8(line[1..].to_vec())?;
            Ok(Some(RespValue::Error(content)))
        } else {
            Ok(None)
        }
    }

    /// 解析整数
    fn parse_integer(buf: &mut BytesMut) -> RedisResult<Option<RespValue>> {
        if let Some(line) = Self::read_line(buf)? {
            let content = String::from_utf8(line[1..].to_vec())?;
            let num: i64 = content.parse()?;
            Ok(Some(RespValue::Integer(num)))
        } else {
            Ok(None)
        }
    }

    /// 解析批量字符串
    ///
    /// Rust特点: 使用if let进行模式匹配和解构
    fn parse_bulk_string(buf: &mut BytesMut) -> RedisResult<Option<RespValue>> {
        // 先尝试读取长度行
        let (len, header_len) = match Self::peek_line(buf)? {
            Some((line, total_len)) => {
                let len_str = String::from_utf8(line[1..].to_vec())?;
                let len: i64 = len_str.parse()?;
                (len, total_len)
            }
            None => return Ok(None),
        };

        // 处理空值
        if len == -1 {
            buf.advance(header_len);
            return Ok(Some(RespValue::Null));
        }

        let len = len as usize;
        let total_needed = header_len + len + 2; // +2 for \r\n

        // 检查是否有足够的数据
        if buf.len() < total_needed {
            return Ok(None);
        }

        // 消费长度行
        buf.advance(header_len);

        // 读取数据
        let data = buf[..len].to_vec();
        buf.advance(len + 2); // 跳过数据和 \r\n

        Ok(Some(RespValue::BulkString(data)))
    }

    /// 解析数组
    ///
    /// Rust特点: 递归调用处理嵌套数组
    fn parse_array(buf: &mut BytesMut) -> RedisResult<Option<RespValue>> {
        let (count, header_len) = match Self::peek_line(buf)? {
            Some((line, total_len)) => {
                let count_str = String::from_utf8(line[1..].to_vec())?;
                let count: i64 = count_str.parse()?;
                (count, total_len)
            }
            None => return Ok(None),
        };

        // 处理空数组
        if count == -1 {
            buf.advance(header_len);
            return Ok(Some(RespValue::Null));
        }

        buf.advance(header_len);

        let count = count as usize;
        let mut items = Vec::with_capacity(count);

        // 递归解析每个元素
        for _ in 0..count {
            match Self::parse(buf)? {
                Some(value) => items.push(value),
                None => {
                    // 数据不完整，需要回滚
                    // 注意：这里简化处理，实际应该保存状态
                    return Err(RedisError::Protocol(
                        "数组数据不完整".to_string(),
                    ));
                }
            }
        }

        Ok(Some(RespValue::Array(items)))
    }

    /// 解析内联命令(简单的文本命令)
    fn parse_inline_command(buf: &mut BytesMut) -> RedisResult<Option<RespValue>> {
        if let Some(line) = Self::read_line(buf)? {
            let content = String::from_utf8(line)?;
            let parts: Vec<RespValue> = content
                .split_whitespace()
                .map(|s| RespValue::BulkString(s.as_bytes().to_vec()))
                .collect();

            if parts.is_empty() {
                return Ok(None);
            }

            Ok(Some(RespValue::Array(parts)))
        } else {
            Ok(None)
        }
    }

    /// 读取一行并从缓冲区移除
    fn read_line(buf: &mut BytesMut) -> RedisResult<Option<Vec<u8>>> {
        if let Some((line, total_len)) = Self::peek_line(buf)? {
            buf.advance(total_len);
            Ok(Some(line))
        } else {
            Ok(None)
        }
    }

    /// 查看一行但不移除
    ///
    /// 返回 (行内容不含\r\n, 总长度含\r\n)
    fn peek_line(buf: &BytesMut) -> RedisResult<Option<(Vec<u8>, usize)>> {
        for i in 0..buf.len() {
            if i + 1 < buf.len() && buf[i] == b'\r' && buf[i + 1] == b'\n' {
                return Ok(Some((buf[..i].to_vec(), i + 2)));
            }
        }
        Ok(None)
    }
}

/// 便捷函数：创建OK响应
pub fn ok() -> RespValue {
    RespValue::SimpleString("OK".to_string())
}

/// 便捷函数：创建PONG响应
pub fn pong() -> RespValue {
    RespValue::SimpleString("PONG".to_string())
}

/// 便捷函数：创建错误响应
pub fn error(msg: &str) -> RespValue {
    RespValue::Error(msg.to_string())
}

/// 便捷函数：从字符串创建批量字符串
pub fn bulk_string(s: &str) -> RespValue {
    RespValue::BulkString(s.as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_simple_string() {
        let value = RespValue::SimpleString("OK".to_string());
        assert_eq!(value.serialize(), b"+OK\r\n");
    }

    #[test]
    fn test_serialize_integer() {
        let value = RespValue::Integer(42);
        assert_eq!(value.serialize(), b":42\r\n");
    }

    #[test]
    fn test_serialize_bulk_string() {
        let value = RespValue::BulkString(b"hello".to_vec());
        assert_eq!(value.serialize(), b"$5\r\nhello\r\n");
    }

    #[test]
    fn test_serialize_array() {
        let value = RespValue::Array(vec![
            RespValue::BulkString(b"SET".to_vec()),
            RespValue::BulkString(b"key".to_vec()),
            RespValue::BulkString(b"value".to_vec()),
        ]);
        assert_eq!(
            value.serialize(),
            b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n"
        );
    }

    #[test]
    fn test_parse_simple_string() {
        let mut buf = BytesMut::from(&b"+OK\r\n"[..]);
        let result = RespParser::parse(&mut buf).unwrap().unwrap();
        assert_eq!(result, RespValue::SimpleString("OK".to_string()));
    }

    #[test]
    fn test_parse_integer() {
        let mut buf = BytesMut::from(&b":1000\r\n"[..]);
        let result = RespParser::parse(&mut buf).unwrap().unwrap();
        assert_eq!(result, RespValue::Integer(1000));
    }
}

