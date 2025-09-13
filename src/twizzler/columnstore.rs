use crate::twizzler::value::TwzValue;
use twizzler::marker::Invariant;

pub const MAX_COLUMNS: usize = 4;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct ColumnStore {
    column_count: u32,
    columns: [TwzValue; MAX_COLUMNS],
}

impl ColumnStore {
    pub fn new() -> Self {
        Self {
            column_count: 0,
            columns: [TwzValue::Null; MAX_COLUMNS],
        }
    }
    
    pub fn from_values(values: &[TwzValue]) -> Result<Self, &'static str> {
        if values.len() > MAX_COLUMNS {
            return Err("Too many columns for fixed storage");
        }
        
        let mut columns = [TwzValue::Null; MAX_COLUMNS];
        columns[..values.len()].copy_from_slice(values);
        
        Ok(Self {
            column_count: values.len() as u32,
            columns,
        })
    }
    
    pub fn set_column(&mut self, index: usize, value: TwzValue) -> Result<(), &'static str> {
        if index >= MAX_COLUMNS {
            return Err("Column index out of bounds");
        }
        
        self.columns[index] = value;
        if index as u32 >= self.column_count {
            self.column_count = (index + 1) as u32;
        }
        
        Ok(())
    }
    
    pub fn get_column(&self, index: usize) -> Option<&TwzValue> {
        if index < self.column_count as usize {
            Some(&self.columns[index])
        } else {
            None
        }
    }
    
    pub fn columns(&self) -> &[TwzValue] {
        &self.columns[..self.column_count as usize]
    }
    
    pub fn column_count(&self) -> usize {
        self.column_count as usize
    }
    
    pub fn clear(&mut self) {
        self.column_count = 0;
        self.columns = [TwzValue::Null; MAX_COLUMNS];
    }
}

unsafe impl Invariant for ColumnStore {}