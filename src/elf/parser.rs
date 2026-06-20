#![allow(dead_code)]

use crate::elf::{Endian, SymbolEntry, SymbolList};
use anyhow::{bail, Context, Result};
use object::{Object, ObjectSection, ObjectSymbol, SectionKind, SymbolKind};
use std::fs;
use std::path::Path;

pub enum ScanMode {
    Convention { prefix: &'static str },
    AllSections,
}

impl SymbolList {
    pub fn scan_file(path: &Path, mode: &ScanMode) -> Result<SymbolList> {
        let data = fs::read(path)?;
        let obj = object::File::parse(&*data)?;
        check_not_stripped(&obj)?;

        let endian = endian_of(&obj);
        let mut list = SymbolList::default();

        for sym in obj.symbols() {
            if !is_candidate(&sym, mode) {
                continue;
            }
            let Some(section) = find_section(&obj, &sym) else {
                continue;
            };

            // .bss
            if section.kind() == SectionKind::UninitializedData {
                list.bss.push(SymbolEntry {
                    name: sym.name().unwrap_or("<unknown>").to_string(),
                    raw_bytes: vec![0u8; sym.size() as usize],
                    file_offset: 0, 
                    size: sym.size() as usize,
                    section_name: section.name().unwrap_or("<unknown>").to_string(),
                    endian,
                });
                continue;
            }

            let Ok(entry) = build_entry(&sym, &section, endian) else {
                continue;
            };
            push_into_group(&mut list, &section, entry);
        }

        sort_all(&mut list);
        Ok(list)
    }
}

// Check that the symbol table exists. A stripped binary's .symtab will be
// completely empty, so we exit early with a clear error message if this happens.
fn check_not_stripped(obj: &object::File) -> Result<()> {
    if obj.symbols().count() == 0 {
        bail!(
            "could not find symbol table (strip may have removed .symtab)"
        );
    }
    Ok(())
}

fn endian_of(obj: &object::File) -> Endian {
    if obj.is_little_endian() {
        Endian::Little
    } else {
        Endian::Big
    }
}

fn is_candidate(sym: &object::Symbol, mode: &ScanMode) -> bool {
    if sym.kind() != SymbolKind::Data {
        return false;
    }
    if sym.size() == 0 {
        return false;
    }
    let name = match sym.name() {
        Ok(name) if !name.is_empty() => name,
        _ => return false,
    };
    match mode {
        ScanMode::Convention { prefix } => name.starts_with(prefix),
        // --all mode does not filter noise symbols here (e.g. .L prefixed compiler internal labels)
        // parser layer is responsible for emitting complete, honest data, 
        // while CLI layer is responsible for filtering noise symbols
        ScanMode::AllSections => true,
    }
}

fn find_section<'data, 'file>(
    obj: &'file object::File<'data>,
    sym: &object::Symbol<'data, 'file>,
) -> Option<object::Section<'data, 'file>> {
    let index = sym.section().index()?;
    obj.section_by_index(index).ok()
}

fn build_entry(
    sym: &object::Symbol,
    section: &object::Section,
    endian: Endian,
) -> Result<SymbolEntry> {
    let sym_addr = sym.address();
    let section_addr = section.address();
    let (section_file_offset, _) = section
        .file_range()
        .context("no file range for section")?;

    let relative = sym_addr - section_addr;
    let file_offset = section_file_offset + relative;

    let size = sym.size() as usize;
    let section_data = section.data().context("could not read section data")?;
    let start = relative as usize;
    let end = start + size;
    let raw_bytes = section_data
        .get(start..end)
        .context("symbol exceed the boundary of section data")?
        .to_vec();

    Ok(SymbolEntry {
        name: sym.name().unwrap_or("<unknown>").to_string(),
        raw_bytes,
        file_offset,
        size,
        section_name: section.name().unwrap_or("<unknown>").to_string(),
        endian,
    })
}

// group by section kind
fn push_into_group(list: &mut SymbolList, section: &object::Section, entry: SymbolEntry) {
    match section.kind() {
        SectionKind::Data => list.data.push(entry),
        SectionKind::ReadOnlyData | SectionKind::ReadOnlyDataWithRel => list.rodata.push(entry),
        _ => list.other.push(entry),
    }
}

fn sort_all(list: &mut SymbolList) {
    list.data.sort_by(|a, b| a.name.cmp(&b.name));
    list.rodata.sort_by(|a, b| a.name.cmp(&b.name));
    list.bss.sort_by(|a, b| a.name.cmp(&b.name));
    list.other.sort_by(|a, b| a.name.cmp(&b.name));
}
