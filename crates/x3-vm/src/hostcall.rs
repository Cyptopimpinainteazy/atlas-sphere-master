//! Hostcall Registry
//!
//! Hostcalls are external functions that can be invoked from X3 bytecode.
//! They provide an interface between the VM and the host environment
//! (blockchain runtime, sidecar, or local development server).

use std::collections::BTreeMap;

use crate::error::{VMError, VMErrorKind, VMResult};
use crate::vm::Value;

/// Hostcall function signature.
pub type HostcallFn = Box<dyn Fn(&[Value]) -> VMResult<Option<Value>> + Send + Sync>;

/// A hostcall registration.
pub struct Hostcall {
    /// Unique hostcall ID (0-255).
    pub id: u8,
    /// Human-readable name.
    pub name: String,
    /// Expected argument count.
    pub arg_count: usize,
    /// The implementation.
    pub handler: HostcallFn,
}

/// Registry of hostcalls available to the VM.
pub struct HostcallRegistry {
    calls: BTreeMap<u8, Hostcall>,
}

impl HostcallRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            calls: BTreeMap::new(),
        }
    }

    /// Create a registry with standard hostcalls.
    pub fn with_standard() -> Self {
        let mut registry = Self::new();
        registry.register_standard_hostcalls();
        registry
    }

    /// Register a hostcall.
    pub fn register<F>(&mut self, id: u8, name: impl Into<String>, arg_count: usize, handler: F)
    where
        F: Fn(&[Value]) -> VMResult<Option<Value>> + Send + Sync + 'static,
    {
        self.calls.insert(
            id,
            Hostcall {
                id,
                name: name.into(),
                arg_count,
                handler: Box::new(handler),
            },
        );
    }

    /// Look up a hostcall by ID.
    pub fn get(&self, id: u8) -> Option<&Hostcall> {
        self.calls.get(&id)
    }

    /// Invoke a hostcall by ID.
    pub fn invoke(&self, id: u8, args: &[Value]) -> VMResult<Option<Value>> {
        let call = self
            .calls
            .get(&id)
            .ok_or_else(|| VMError::without_ip(VMErrorKind::HostcallNotFound(id)))?;

        if args.len() != call.arg_count {
            return Err(VMError::without_ip(VMErrorKind::ArgumentCountMismatch(
                call.arg_count,
                args.len(),
            )));
        }

        (call.handler)(args)
    }

    /// Register standard development hostcalls.
    fn register_standard_hostcalls(&mut self) {
        // ID 0: log_debug - print debug message
        self.register(0, "log_debug", 1, |args| {
            if let Some(Value::String(s)) = args.first() {
                log::debug!("[X3] {}", s);
            } else if let Some(v) = args.first() {
                log::debug!("[X3] {:?}", v);
            }
            Ok(None)
        });

        // ID 1: log_info - print info message
        self.register(1, "log_info", 1, |args| {
            if let Some(Value::String(s)) = args.first() {
                log::info!("[X3] {}", s);
            } else if let Some(v) = args.first() {
                log::info!("[X3] {:?}", v);
            }
            Ok(None)
        });

        // ID 2: log_error - print error message
        self.register(2, "log_error", 1, |args| {
            if let Some(Value::String(s)) = args.first() {
                log::error!("[X3] {}", s);
            } else if let Some(v) = args.first() {
                log::error!("[X3] {:?}", v);
            }
            Ok(None)
        });

        // ID 3: assert_true - assertion check
        self.register(3, "assert_true", 1, |args| match args.first() {
            Some(Value::Bool(true)) => Ok(None),
            Some(Value::Bool(false)) => Err(VMError::without_ip(VMErrorKind::UserPanic(
                "assertion failed".to_string(),
            ))),
            _ => Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                "bool".to_string(),
                "other".to_string(),
            ))),
        });

        // ID 4: assert_eq - equality assertion
        self.register(4, "assert_eq", 2, |args| {
            if args.len() != 2 {
                return Err(VMError::without_ip(VMErrorKind::ArgumentCountMismatch(
                    2,
                    args.len(),
                )));
            }
            if args[0] == args[1] {
                Ok(None)
            } else {
                Err(VMError::without_ip(VMErrorKind::UserPanic(format!(
                    "assertion failed: {:?} != {:?}",
                    args[0], args[1]
                ))))
            }
        });

        // ID 5: panic - trigger panic with message
        self.register(5, "panic", 1, |args| {
            let msg = match args.first() {
                Some(Value::String(s)) => s.clone(),
                Some(v) => format!("{:?}", v),
                None => "panic".to_string(),
            };
            Err(VMError::without_ip(VMErrorKind::UserPanic(msg)))
        });

        // ID 10: get_timestamp - get current timestamp
        self.register(10, "get_timestamp", 0, |_args| {
            use std::time::{SystemTime, UNIX_EPOCH};
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            Ok(Some(Value::I64(ts)))
        });

        // ID 11: get_random - get pseudo-random number (NOT for production!)
        self.register(11, "get_random", 0, |_args| {
            use std::time::{SystemTime, UNIX_EPOCH};
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as i64)
                .unwrap_or(0);
            // Simple LCG for testing only
            Ok(Some(Value::I64(
                ts.wrapping_mul(1103515245).wrapping_add(12345),
            )))
        });

        // ID 20: print_i64 - print i64 value
        self.register(20, "print_i64", 1, |args| {
            if let Some(Value::I64(v)) = args.first() {
                println!("{}", v);
            }
            Ok(None)
        });

        // ID 21: print_f64 - print f64 value
        self.register(21, "print_f64", 1, |args| {
            if let Some(Value::F64(v)) = args.first() {
                println!("{}", v);
            }
            Ok(None)
        });

        // ID 22: print_string - print string value
        self.register(22, "print_string", 1, |args| {
            if let Some(Value::String(s)) = args.first() {
                println!("{}", s);
            }
            Ok(None)
        });
    }
}

impl Default for HostcallRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard hostcall IDs.
pub mod hostcall_ids {
    pub const LOG_DEBUG: u8 = 0;
    pub const LOG_INFO: u8 = 1;
    pub const LOG_ERROR: u8 = 2;
    pub const ASSERT_TRUE: u8 = 3;
    pub const ASSERT_EQ: u8 = 4;
    pub const PANIC: u8 = 5;
    pub const GET_TIMESTAMP: u8 = 10;
    pub const GET_RANDOM: u8 = 11;
    pub const PRINT_I64: u8 = 20;
    pub const PRINT_F64: u8 = 21;
    pub const PRINT_STRING: u8 = 22;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_standard() {
        let registry = HostcallRegistry::with_standard();
        assert!(registry.get(0).is_some()); // log_debug
        assert!(registry.get(3).is_some()); // assert_true
    }

    #[test]
    fn invoke_assert_true_pass() {
        let registry = HostcallRegistry::with_standard();
        let result = registry.invoke(3, &[Value::Bool(true)]);
        assert!(result.is_ok());
    }

    #[test]
    fn invoke_assert_true_fail() {
        let registry = HostcallRegistry::with_standard();
        let result = registry.invoke(3, &[Value::Bool(false)]);
        assert!(result.is_err());
    }

    #[test]
    fn invoke_assert_eq_pass() {
        let registry = HostcallRegistry::with_standard();
        let result = registry.invoke(4, &[Value::I64(42), Value::I64(42)]);
        assert!(result.is_ok());
    }

    #[test]
    fn invoke_assert_eq_fail() {
        let registry = HostcallRegistry::with_standard();
        let result = registry.invoke(4, &[Value::I64(42), Value::I64(43)]);
        assert!(result.is_err());
    }

    #[test]
    fn invoke_unknown_hostcall() {
        let registry = HostcallRegistry::new();
        let result = registry.invoke(255, &[]);
        assert!(matches!(
            result.unwrap_err().kind,
            VMErrorKind::HostcallNotFound(255)
        ));
    }
}
