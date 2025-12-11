//! 命令处理模块 - 展示Rust的trait和模式匹配
//!
//! Rust特点展示:
//! - 枚举表示不同命令类型
//! - trait定义命令执行接口
//! - 模式匹配解析和执行命令
//! - 生命周期标注

use crate::error::{RedisError, RedisResult};
use crate::resp::{self, RespValue};
use crate::store::Store;
use std::time::Duration;

/// Redis命令枚举
///
/// Rust特点: 枚举的每个变体可以携带不同的数据
#[derive(Debug, Clone)]
pub enum Command {
    // 连接命令
    Ping(Option<String>),
    Echo(String),
    Quit,

    // 字符串命令
    Get { key: String },
    Set {
        key: String,
        value: Vec<u8>,
        expiry: Option<Duration>,
        nx: bool, // 仅当键不存在时设置
        xx: bool, // 仅当键存在时设置
    },
    GetSet { key: String, value: Vec<u8> },
    Append { key: String, value: Vec<u8> },
    Strlen { key: String },
    Incr { key: String },
    IncrBy { key: String, delta: i64 },
    Decr { key: String },
    DecrBy { key: String, delta: i64 },
    MGet { keys: Vec<String> },
    MSet { pairs: Vec<(String, Vec<u8>)> },

    // 键命令
    Del { keys: Vec<String> },
    Exists { keys: Vec<String> },
    Expire { key: String, seconds: u64 },
    PExpire { key: String, milliseconds: u64 },
    Ttl { key: String },
    PTtl { key: String },
    Persist { key: String },
    Keys { pattern: String },
    Type { key: String },
    Rename { old_key: String, new_key: String },

    // 服务器命令
    DbSize,
    FlushDb,
    Info,

    // 未知命令
    Unknown(String),
}

impl Command {
    /// 从RESP值解析命令
    ///
    /// Rust特点: 强大的模式匹配，可以同时匹配和解构
    pub fn from_resp(value: RespValue) -> RedisResult<Command> {
        // 获取命令数组
        let parts = match value {
            RespValue::Array(arr) => arr,
            _ => return Err(RedisError::Protocol("期望数组".to_string())),
        };

        if parts.is_empty() {
            return Err(RedisError::Protocol("空命令".to_string()));
        }

        // 获取命令名称并转为大写
        let cmd_name = parts[0]
            .as_string()
            .ok_or_else(|| RedisError::Protocol("命令名必须是字符串".to_string()))?
            .to_uppercase();

        // 获取参数
        let args: Vec<RespValue> = parts.into_iter().skip(1).collect();

        // 根据命令名称解析
        Self::parse_command(&cmd_name, args)
    }

    /// 解析具体命令
    ///
    /// Rust特点: match表达式返回值，所有分支必须返回相同类型
    fn parse_command(cmd: &str, args: Vec<RespValue>) -> RedisResult<Command> {
        match cmd {
            // ===== 连接命令 =====
            "PING" => {
                let msg = args.first().and_then(|v| v.as_string());
                Ok(Command::Ping(msg))
            }

            "ECHO" => {
                Self::require_args("ECHO", &args, 1)?;
                let msg = args[0]
                    .as_string()
                    .ok_or_else(|| RedisError::TypeError("参数必须是字符串".to_string()))?;
                Ok(Command::Echo(msg))
            }

            "QUIT" => Ok(Command::Quit),

            // ===== 字符串命令 =====
            "GET" => {
                Self::require_args("GET", &args, 1)?;
                let key = Self::get_string(&args[0])?;
                Ok(Command::Get { key })
            }

            "SET" => {
                Self::require_min_args("SET", &args, 2)?;
                let key = Self::get_string(&args[0])?;
                let value = Self::get_bytes(&args[1])?;

                // 解析可选参数
                let mut expiry = None;
                let mut nx = false;
                let mut xx = false;
                let mut i = 2;

                while i < args.len() {
                    let opt = args[i]
                        .as_string()
                        .ok_or_else(|| RedisError::Protocol("无效的选项".to_string()))?
                        .to_uppercase();

                    match opt.as_str() {
                        "EX" => {
                            i += 1;
                            let secs = Self::get_integer(&args[i])?;
                            expiry = Some(Duration::from_secs(secs as u64));
                        }
                        "PX" => {
                            i += 1;
                            let ms = Self::get_integer(&args[i])?;
                            expiry = Some(Duration::from_millis(ms as u64));
                        }
                        "NX" => nx = true,
                        "XX" => xx = true,
                        _ => {
                            return Err(RedisError::Protocol(format!("未知选项: {}", opt)));
                        }
                    }
                    i += 1;
                }

                Ok(Command::Set {
                    key,
                    value,
                    expiry,
                    nx,
                    xx,
                })
            }

            "GETSET" => {
                Self::require_args("GETSET", &args, 2)?;
                Ok(Command::GetSet {
                    key: Self::get_string(&args[0])?,
                    value: Self::get_bytes(&args[1])?,
                })
            }

            "APPEND" => {
                Self::require_args("APPEND", &args, 2)?;
                Ok(Command::Append {
                    key: Self::get_string(&args[0])?,
                    value: Self::get_bytes(&args[1])?,
                })
            }

            "STRLEN" => {
                Self::require_args("STRLEN", &args, 1)?;
                Ok(Command::Strlen {
                    key: Self::get_string(&args[0])?,
                })
            }

            "INCR" => {
                Self::require_args("INCR", &args, 1)?;
                Ok(Command::Incr {
                    key: Self::get_string(&args[0])?,
                })
            }

            "INCRBY" => {
                Self::require_args("INCRBY", &args, 2)?;
                Ok(Command::IncrBy {
                    key: Self::get_string(&args[0])?,
                    delta: Self::get_integer(&args[1])?,
                })
            }

            "DECR" => {
                Self::require_args("DECR", &args, 1)?;
                Ok(Command::Decr {
                    key: Self::get_string(&args[0])?,
                })
            }

            "DECRBY" => {
                Self::require_args("DECRBY", &args, 2)?;
                Ok(Command::DecrBy {
                    key: Self::get_string(&args[0])?,
                    delta: Self::get_integer(&args[1])?,
                })
            }

            "MGET" => {
                Self::require_min_args("MGET", &args, 1)?;
                let keys: Result<Vec<_>, _> = args.iter().map(Self::get_string).collect();
                Ok(Command::MGet { keys: keys? })
            }

            "MSET" => {
                if args.len() < 2 || args.len() % 2 != 0 {
                    return Err(RedisError::WrongNumberOfArguments {
                        command: "MSET".to_string(),
                        expected: 2,
                        got: args.len(),
                    });
                }
                let mut pairs = Vec::new();
                for chunk in args.chunks(2) {
                    pairs.push((Self::get_string(&chunk[0])?, Self::get_bytes(&chunk[1])?));
                }
                Ok(Command::MSet { pairs })
            }

            // ===== 键命令 =====
            "DEL" => {
                Self::require_min_args("DEL", &args, 1)?;
                let keys: Result<Vec<_>, _> = args.iter().map(Self::get_string).collect();
                Ok(Command::Del { keys: keys? })
            }

            "EXISTS" => {
                Self::require_min_args("EXISTS", &args, 1)?;
                let keys: Result<Vec<_>, _> = args.iter().map(Self::get_string).collect();
                Ok(Command::Exists { keys: keys? })
            }

            "EXPIRE" => {
                Self::require_args("EXPIRE", &args, 2)?;
                Ok(Command::Expire {
                    key: Self::get_string(&args[0])?,
                    seconds: Self::get_integer(&args[1])? as u64,
                })
            }

            "PEXPIRE" => {
                Self::require_args("PEXPIRE", &args, 2)?;
                Ok(Command::PExpire {
                    key: Self::get_string(&args[0])?,
                    milliseconds: Self::get_integer(&args[1])? as u64,
                })
            }

            "TTL" => {
                Self::require_args("TTL", &args, 1)?;
                Ok(Command::Ttl {
                    key: Self::get_string(&args[0])?,
                })
            }

            "PTTL" => {
                Self::require_args("PTTL", &args, 1)?;
                Ok(Command::PTtl {
                    key: Self::get_string(&args[0])?,
                })
            }

            "PERSIST" => {
                Self::require_args("PERSIST", &args, 1)?;
                Ok(Command::Persist {
                    key: Self::get_string(&args[0])?,
                })
            }

            "KEYS" => {
                Self::require_args("KEYS", &args, 1)?;
                Ok(Command::Keys {
                    pattern: Self::get_string(&args[0])?,
                })
            }

            "TYPE" => {
                Self::require_args("TYPE", &args, 1)?;
                Ok(Command::Type {
                    key: Self::get_string(&args[0])?,
                })
            }

            "RENAME" => {
                Self::require_args("RENAME", &args, 2)?;
                Ok(Command::Rename {
                    old_key: Self::get_string(&args[0])?,
                    new_key: Self::get_string(&args[1])?,
                })
            }

            // ===== 服务器命令 =====
            "DBSIZE" => Ok(Command::DbSize),

            "FLUSHDB" | "FLUSHALL" => Ok(Command::FlushDb),

            "INFO" => Ok(Command::Info),

            // 未知命令
            _ => Ok(Command::Unknown(cmd.to_string())),
        }
    }

    /// 检查参数数量是否正确
    fn require_args(cmd: &str, args: &[RespValue], expected: usize) -> RedisResult<()> {
        if args.len() != expected {
            Err(RedisError::WrongNumberOfArguments {
                command: cmd.to_string(),
                expected,
                got: args.len(),
            })
        } else {
            Ok(())
        }
    }

    /// 检查最少参数数量
    fn require_min_args(cmd: &str, args: &[RespValue], min: usize) -> RedisResult<()> {
        if args.len() < min {
            Err(RedisError::WrongNumberOfArguments {
                command: cmd.to_string(),
                expected: min,
                got: args.len(),
            })
        } else {
            Ok(())
        }
    }

    /// 从RESP值获取字符串
    fn get_string(value: &RespValue) -> RedisResult<String> {
        value
            .as_string()
            .ok_or_else(|| RedisError::TypeError("期望字符串".to_string()))
    }

    /// 从RESP值获取字节
    fn get_bytes(value: &RespValue) -> RedisResult<Vec<u8>> {
        match value {
            RespValue::BulkString(data) => Ok(data.clone()),
            RespValue::SimpleString(s) => Ok(s.as_bytes().to_vec()),
            _ => Err(RedisError::TypeError("期望字符串".to_string())),
        }
    }

    /// 从RESP值获取整数
    fn get_integer(value: &RespValue) -> RedisResult<i64> {
        value
            .as_integer()
            .ok_or_else(|| RedisError::TypeError("期望整数".to_string()))
    }
}

/// 命令执行器 - 实现命令执行逻辑
///
/// Rust特点: 结构体方法实现业务逻辑
pub struct CommandExecutor<'a> {
    store: &'a Store,
}

impl<'a> CommandExecutor<'a> {
    /// 创建新的执行器
    ///
    /// Rust特点: 生命周期'a确保执行器不会比store活得更久
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    /// 执行命令并返回响应
    ///
    /// Rust特点: 穷尽的模式匹配确保所有命令都被处理
    pub fn execute(&self, cmd: Command) -> (RespValue, bool) {
        let should_quit = matches!(cmd, Command::Quit);

        let response = match cmd {
            // 连接命令
            Command::Ping(msg) => match msg {
                Some(m) => resp::bulk_string(&m),
                None => resp::pong(),
            },

            Command::Echo(msg) => resp::bulk_string(&msg),

            Command::Quit => resp::ok(),

            // 字符串命令
            Command::Get { key } => match self.store.get(&key) {
                Some(data) => RespValue::BulkString(data),
                None => RespValue::Null,
            },

            Command::Set {
                key,
                value,
                expiry,
                nx,
                xx,
            } => {
                // NX: 只在键不存在时设置
                // XX: 只在键存在时设置
                let exists = self.store.exists(&key);

                if (nx && exists) || (xx && !exists) {
                    RespValue::Null
                } else {
                    match expiry {
                        Some(ttl) => self.store.set_with_expiry(key, value, ttl),
                        None => self.store.set(key, value),
                    }
                    resp::ok()
                }
            }

            Command::GetSet { key, value } => {
                let old = self.store.get(&key);
                self.store.set(key, value);
                match old {
                    Some(data) => RespValue::BulkString(data),
                    None => RespValue::Null,
                }
            }

            Command::Append { key, value } => {
                let len = self.store.append(&key, &value);
                RespValue::Integer(len as i64)
            }

            Command::Strlen { key } => {
                let len = self.store.strlen(&key);
                RespValue::Integer(len as i64)
            }

            Command::Incr { key } => match self.store.incr(&key, 1) {
                Ok(n) => RespValue::Integer(n),
                Err(e) => resp::error(&e),
            },

            Command::IncrBy { key, delta } => match self.store.incr(&key, delta) {
                Ok(n) => RespValue::Integer(n),
                Err(e) => resp::error(&e),
            },

            Command::Decr { key } => match self.store.incr(&key, -1) {
                Ok(n) => RespValue::Integer(n),
                Err(e) => resp::error(&e),
            },

            Command::DecrBy { key, delta } => match self.store.incr(&key, -delta) {
                Ok(n) => RespValue::Integer(n),
                Err(e) => resp::error(&e),
            },

            Command::MGet { keys } => {
                let values: Vec<RespValue> = keys
                    .iter()
                    .map(|k| match self.store.get(k) {
                        Some(data) => RespValue::BulkString(data),
                        None => RespValue::Null,
                    })
                    .collect();
                RespValue::Array(values)
            }

            Command::MSet { pairs } => {
                for (key, value) in pairs {
                    self.store.set(key, value);
                }
                resp::ok()
            }

            // 键命令
            Command::Del { keys } => {
                let count = self.store.del_multi(&keys);
                RespValue::Integer(count as i64)
            }

            Command::Exists { keys } => {
                let count = self.store.exists_multi(&keys);
                RespValue::Integer(count as i64)
            }

            Command::Expire { key, seconds } => {
                let success = self.store.expire(&key, Duration::from_secs(seconds));
                RespValue::Integer(if success { 1 } else { 0 })
            }

            Command::PExpire { key, milliseconds } => {
                let success = self.store.expire(&key, Duration::from_millis(milliseconds));
                RespValue::Integer(if success { 1 } else { 0 })
            }

            Command::Ttl { key } => {
                let ttl_ms = self.store.pttl(&key);
                let ttl_s = if ttl_ms > 0 {
                    ttl_ms / 1000
                } else {
                    ttl_ms
                };
                RespValue::Integer(ttl_s)
            }

            Command::PTtl { key } => {
                let ttl = self.store.pttl(&key);
                RespValue::Integer(ttl)
            }

            Command::Persist { key } => {
                let success = self.store.persist(&key);
                RespValue::Integer(if success { 1 } else { 0 })
            }

            Command::Keys { pattern } => {
                let keys = self.store.keys(&pattern);
                RespValue::Array(
                    keys.into_iter()
                        .map(|k| RespValue::BulkString(k.into_bytes()))
                        .collect(),
                )
            }

            Command::Type { key } => match self.store.key_type(&key) {
                Some(t) => RespValue::SimpleString(t.to_string()),
                None => RespValue::SimpleString("none".to_string()),
            },

            Command::Rename { old_key, new_key } => {
                if self.store.rename(&old_key, &new_key) {
                    resp::ok()
                } else {
                    resp::error("ERR no such key")
                }
            }

            // 服务器命令
            Command::DbSize => RespValue::Integer(self.store.dbsize() as i64),

            Command::FlushDb => {
                self.store.flushdb();
                resp::ok()
            }

            Command::Info => {
                let info = format!(
                    "# Server\r\n\
                     redis_version:0.1.0\r\n\
                     rust_version:{}\r\n\
                     # Keyspace\r\n\
                     db0:keys={}\r\n",
                    env!("CARGO_PKG_VERSION"),
                    self.store.dbsize()
                );
                RespValue::BulkString(info.into_bytes())
            }

            Command::Unknown(cmd) => {
                resp::error(&format!("ERR unknown command '{}'", cmd))
            }
        };

        (response, should_quit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ping() {
        let value = RespValue::Array(vec![RespValue::BulkString(b"PING".to_vec())]);
        let cmd = Command::from_resp(value).unwrap();
        assert!(matches!(cmd, Command::Ping(None)));
    }

    #[test]
    fn test_parse_set() {
        let value = RespValue::Array(vec![
            RespValue::BulkString(b"SET".to_vec()),
            RespValue::BulkString(b"key".to_vec()),
            RespValue::BulkString(b"value".to_vec()),
        ]);
        let cmd = Command::from_resp(value).unwrap();
        assert!(matches!(cmd, Command::Set { .. }));
    }

    #[test]
    fn test_execute_ping() {
        let store = Store::new();
        let executor = CommandExecutor::new(&store);
        let (response, _) = executor.execute(Command::Ping(None));
        assert_eq!(response, RespValue::SimpleString("PONG".to_string()));
    }

    #[test]
    fn test_execute_set_get() {
        let store = Store::new();
        let executor = CommandExecutor::new(&store);

        // SET
        let (response, _) = executor.execute(Command::Set {
            key: "foo".to_string(),
            value: b"bar".to_vec(),
            expiry: None,
            nx: false,
            xx: false,
        });
        assert_eq!(response, RespValue::SimpleString("OK".to_string()));

        // GET
        let (response, _) = executor.execute(Command::Get {
            key: "foo".to_string(),
        });
        assert_eq!(response, RespValue::BulkString(b"bar".to_vec()));
    }
}

