use crate::types::{ToSql, ToSqlOutput, Value, ValueRef};
use twizzler::marker::Invariant;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum TwzValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(FixedString<256>), // Fixed length string for now
    Blob([u8; 1024]), // Fixed length blob for now
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct FixedString<const N: usize> {
    data: [u8; N],
    len: u64,
}

impl<const N: usize> FixedString<N> {
    fn new(data: &[u8]) -> Self {
        let mut buf = [0; N];
        let len = data.len().min(N);
        buf[..len].copy_from_slice(&data[..len]);
        Self { data: buf, len: len as u64 }
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.data[..self.len as usize]).unwrap_or("")
    }

    fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }

    fn from_str(s: &str) -> Self {
        Self::new(s.as_bytes())
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self::new(bytes)
    }
}

impl From<ValueRef<'_>> for TwzValue {
    fn from(v: ValueRef<'_>) -> Self {
        match v {
            ValueRef::Null => TwzValue::Null,
            ValueRef::Integer(i) => TwzValue::Integer(i),
            ValueRef::Real(f) => TwzValue::Real(f),
            ValueRef::Text(t) => TwzValue::Text(FixedString::from_bytes(t)),
            ValueRef::Blob(b) => {
                let mut buf = [0; 1024];
                let len = b.len().min(1024);
                buf[..len].copy_from_slice(&b[..len]);
                TwzValue::Blob(buf)
            },
        }
    }
}

impl From<Value> for TwzValue {
    fn from(v: Value) -> Self {
        match v {
            Value::Null => TwzValue::Null,
            Value::Integer(i) => TwzValue::Integer(i),
            Value::Real(f) => TwzValue::Real(f),
            Value::Text(t) => TwzValue::Text(FixedString::from_str(&t)),
            Value::Blob(b) => {
                let mut buf = [0; 1024];
                let len = b.len().min(1024);
                buf[..len].copy_from_slice(&b[..len]);
                TwzValue::Blob(buf)
            },
        }
    }
}

impl From<TwzValue> for Value {
    fn from(tv: TwzValue) -> Self {
        match tv {
            TwzValue::Null => Value::Null,
            TwzValue::Integer(i) => Value::Integer(i),
            TwzValue::Real(f) => Value::Real(f),
            TwzValue::Text(fs) => Value::Text(fs.as_str().to_string()),
            TwzValue::Blob(b) => {
                // Find the actual length of the blob (until first zero or end)
                let len = b.iter().position(|&x| x == 0).unwrap_or(b.len());
                Value::Blob(b[..len].to_vec())
            },
        }
    }
}

impl ToSql for TwzValue {
    fn to_sql(&self) -> crate::Result<ToSqlOutput<'_>> {
        match self {
            TwzValue::Null => Ok(ToSqlOutput::Owned(Value::Null)),
            TwzValue::Integer(i) => Ok(ToSqlOutput::Owned(Value::Integer(*i))),
            TwzValue::Real(f) => Ok(ToSqlOutput::Owned(Value::Real(*f))),
            TwzValue::Text(fs) => Ok(ToSqlOutput::Owned(Value::Text(fs.as_str().to_string()))),
            TwzValue::Blob(b) => {
                let len = b.iter().position(|&x| x == 0).unwrap_or(b.len());
                Ok(ToSqlOutput::Owned(Value::Blob(b[..len].to_vec())))
            },
        }
    }
}

unsafe impl<const N: usize> Invariant for FixedString<N> {}
unsafe impl Invariant for TwzValue {}