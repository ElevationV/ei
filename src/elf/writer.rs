use crate::elf::SymbolEntry;
use anyhow::Result;

use anyhow::ensure;

pub struct Patch {
    pub name: String,
    pub offset: u64,
    pub data: Vec<u8>,
}

impl Patch {
    pub fn new(entry: &SymbolEntry, new_data: Vec<u8>) -> Result<Self> {
        ensure!(
            new_data.len() <= entry.size,
            "Symbol '{}' with size {} exceeds max size {}",
            entry.name,
            new_data.len(),
            entry.size,
        );
        
        Ok(Patch { 
            name: entry.name.clone(),
            offset: entry.file_offset,
            data: new_data,
             })
    }
    
}