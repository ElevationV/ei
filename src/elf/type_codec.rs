use crate::elf::Endian;
use anyhow::{ensure, Context, Result};

trait IntCodec: Sized {
    fn from_bytes(bytes: &[u8], endian: Endian) -> Self;
    fn to_bytes(&self, endian: Endian) -> Vec<u8>;
}

macro_rules! impl_codec {
    ($($t:ty),*) => {
        $(impl IntCodec for $t {
            fn from_bytes(bytes: &[u8], endian: Endian) -> Self {
                let arr: [u8; std::mem::size_of::<$t>()] = bytes.try_into().unwrap();
                match endian {
                    Endian::Little => <$t>::from_le_bytes(arr),
                    Endian::Big => <$t>::from_be_bytes(arr),
                }
            }
            fn to_bytes(&self, endian: Endian) -> Vec<u8> {
                match endian {
                    Endian::Little => self.to_le_bytes().to_vec(),
                    Endian::Big => self.to_be_bytes().to_vec(),
                }
            }
        })*
    };
}

impl_codec!(u16, i16, u32, i32, u64, i64, f32, f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueForm {
    U8, U16, U32, U64, 
    I8, I16, I32, I64, 
    F32, F64,
    Str,
    Hex,
}

impl ValueForm {
    fn fixed_size(self) -> Option<usize> {
        match self {
            ValueForm::U8  | ValueForm::I8  => Some(1),
            ValueForm::U16 | ValueForm::I16 => Some(2),
            ValueForm::U32 | ValueForm::I32 => Some(4),
            ValueForm::U64 | ValueForm::I64 => Some(8),
            ValueForm::F32 => Some(4),
            ValueForm::F64 => Some(8),
            ValueForm::Str | ValueForm::Hex => None,
        }
    }
}

pub fn available_forms(len: usize) -> Vec<ValueForm> {
    let mut forms = vec![];

    for form in [
        ValueForm::U8, ValueForm::U16, ValueForm::U32, ValueForm::U64,
        ValueForm::I8, ValueForm::I16, ValueForm::I32, ValueForm::I64,
        ValueForm::F32, ValueForm::F64,
    ] {
        if form.fixed_size() == Some(len) {
            forms.push(form);
        }
    }

    forms.push(ValueForm::Str);
    forms.push(ValueForm::Hex);
    forms
}

pub fn default_form() -> ValueForm {
    ValueForm::Hex
}

pub fn decode(bytes: &[u8], form: ValueForm, endian: Endian) -> Result<String> {
    if let Some(expected) = form.fixed_size() {
        ensure!(
            bytes.len() == expected,
            "Expected {expected} bytes, got {}",
            bytes.len()
        );
    }

    Ok(match form {
        ValueForm::U8 => bytes[0].to_string(),
        ValueForm::I8 => (bytes[0] as i8).to_string(),
        ValueForm::U16 => decode_int::<u16>(bytes, endian),
        ValueForm::I16 => decode_int::<i16>(bytes, endian),
        ValueForm::U32 => decode_int::<u32>(bytes, endian),
        ValueForm::I32 => decode_int::<i32>(bytes, endian),
        ValueForm::U64 => decode_int::<u64>(bytes, endian),
        ValueForm::I64 => decode_int::<i64>(bytes, endian),
        ValueForm::F32 => f32::from_bytes(bytes, endian).to_string(),
        ValueForm::F64 => f64::from_bytes(bytes, endian).to_string(),
        ValueForm::Str => {
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            String::from_utf8_lossy(&bytes[..end]).into_owned()
        }
        ValueForm::Hex => bytes.iter().map(|b| format!("{b:02x}")).collect(),
    })
}

fn decode_int<T: IntCodec + ToString>(bytes: &[u8], endian: Endian) -> String {
    T::from_bytes(bytes, endian).to_string()
}

pub fn encode(input: &str, form: ValueForm, endian: Endian) -> Result<Vec<u8>> {
    Ok(match form {
        ValueForm::U8 => vec![input.parse::<u8>().context("Not a valid u8")?],
        ValueForm::I8 => vec![input.parse::<i8>().context("Not a valid i8")? as u8],
        ValueForm::U16 => encode_int::<u16>(input, endian)?,
        ValueForm::I16 => encode_int::<i16>(input, endian)?,
        ValueForm::U32 => encode_int::<u32>(input, endian)?,
        ValueForm::I32 => encode_int::<i32>(input, endian)?,
        ValueForm::U64 => encode_int::<u64>(input, endian)?,
        ValueForm::I64 => encode_int::<i64>(input, endian)?,
        ValueForm::F32 => {
            let v: f32 = input.parse().context("Not a valid f32")?;
            v.to_bytes(endian)
        }
        ValueForm::F64 => {
            let v: f64 = input.parse().context("Not a valid f64")?;
            v.to_bytes(endian)
        }
        ValueForm::Str => {
            let mut b = input.as_bytes().to_vec();
            b.push(0);
            b
        }
        ValueForm::Hex => decode_hex_string(input)?,
    })
}

fn encode_int<T>(input: &str, endian: Endian) -> Result<Vec<u8>>
where
    T: IntCodec + std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    let v: T = input.parse().map_err(|e| anyhow::anyhow!("Failed to parse: {e}"))?;
    Ok(v.to_bytes(endian))
}

fn decode_hex_string(input: &str) -> Result<Vec<u8>> {
    let cleaned = input.trim().replace(' ', "");
    
    // cleaned.len().is_multiple_of(2) <==> cleaned.len() % 2 == 0
    ensure!(cleaned.len().is_multiple_of(2), "Length must be even"); 
    (0..cleaned.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&cleaned[i..i + 2], 16).context("Not a valid hex digit"))
        .collect()
}
