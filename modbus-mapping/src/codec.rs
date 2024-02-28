use std::fmt::Debug;

pub use tokio_modbus::{Address, Quantity};

/// 16-bit value stored in Modbus register.
pub type Word = u16;

#[derive(Debug)]
pub struct WordsCountError {}

/// Decode a value from Big or Little Endian-ordered `Word`s.
pub trait Decode: Sized {
    fn from_be_words(words: &[Word]) -> Result<Self, WordsCountError>;
    fn from_le_words(words: &[Word]) -> Result<Self, WordsCountError>;
}

macro_rules! impl_decode {
    ($num_type:ty) => {
        impl Decode for $num_type {
            fn from_be_words(words: &[Word]) -> Result<Self, WordsCountError> {
                let bytes = words
                    .iter()
                    .copied()
                    .flat_map(u16::to_be_bytes)
                    .collect::<Vec<u8>>();
                let array = bytes.try_into().or(Err(WordsCountError {}))?;
                Ok(<$num_type>::from_be_bytes(array))
            }
            fn from_le_words(_words: &[Word]) -> Result<Self, WordsCountError> {
                todo!()
            }
        }
    };
}

impl_decode!(i16);
impl_decode!(i32);
impl_decode!(i64);
impl_decode!(u16);
impl_decode!(u32);
impl_decode!(u64);
impl_decode!(f32);
impl_decode!(f64);

/// Encode a value into Big or Little Endian-ordered `Word`s.
pub trait Encode {
    fn to_be_words(self) -> Vec<Word>;
    fn to_le_words(self) -> Vec<Word>;
}

macro_rules! impl_encode {
    ($num_type:ty) => {
        impl Encode for $num_type {
            fn to_be_words(self) -> Vec<Word> {
                self.to_be_bytes()
                    .chunks(2)
                    .map(|chunk| {
                        let array = chunk.try_into().expect("unexpected encoding error");
                        u16::from_be_bytes(array)
                    })
                    .collect()
            }
            fn to_le_words(self) -> Vec<Word> {
                todo!()
            }
        }
    };
}

impl_encode!(i16);
impl_encode!(i32);
impl_encode!(i64);
impl_encode!(u16);
impl_encode!(u32);
impl_encode!(u64);
impl_encode!(f32);
impl_encode!(f64);
