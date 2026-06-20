#![allow(dead_code)]

pub mod parser;
pub mod writer;

#[derive(Debug, Clone, Copy)]
pub enum Endian {
    Little,
    Big,
}

#[derive(Debug, Clone)]
pub struct SymbolEntry {
    pub name: String,
    pub raw_bytes: Vec<u8>,
    pub file_offset: u64,
    pub size: usize,
    pub section_name: String,
    pub endian: Endian,
}

#[derive(Debug, Default)]
pub struct SymbolList {
    pub data: Vec<SymbolEntry>,
    pub rodata: Vec<SymbolEntry>,
    pub bss: Vec<SymbolEntry>,
    pub other: Vec<SymbolEntry>,
}

impl SymbolList {
    // all writable symbols —— data + rodata, excluding bss (.bss has no real bytes)
    // and other (section kind is ambiguous, unclear semantics, not included)
    pub fn writable(&self) -> impl Iterator<Item = &SymbolEntry> {
        self.data.iter().chain(self.rodata.iter())
    }

    pub fn find(&self, name: &str) -> Option<&SymbolEntry> {
        self.writable().find(|e| e.name == name)
    }

    pub fn total_count(&self) -> usize {
        self.data.len() + self.rodata.len() + self.bss.len() + self.other.len()
    }
}