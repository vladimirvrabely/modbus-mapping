use crate::utils::{expr_assign_predicate, get_punctuated, panic_no_key, panic_not_literal};
use proc_macro2::{Ident, Span};
use syn::{punctuated::Punctuated, token::Comma, Expr, ExprAssign, Field, Lit, Type};

#[derive(Debug, Clone)]
/// Single entry in modbus register mapping. Parsed from field attributes and to be used in proc macros
pub struct Entry {
    pub field_name: String,
    pub field_ty: String,
    pub addr: Address,
    pub ty: DataType,
    pub ord: WordOrder,
    pub x: ScaleFactor,
    pub unit: String,
}

pub type Address = u16;
pub type Quantity = u16;

#[derive(Debug, Clone, Copy)]
pub enum DataType {
    U16,
    U32,
    U64,
    I16,
    I32,
    I64,
    F32,
    F64,
    Raw(Quantity),
}

#[derive(Debug, Clone, Copy)]
pub enum WordOrder {
    BigEndian,
    LittleEndian,
}

pub type ScaleFactor = f64;

impl From<String> for DataType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "u16" => DataType::U16,
            "u32" => DataType::U32,
            "u64" => DataType::U64,
            "i16" => DataType::I16,
            "i32" => DataType::I32,
            "i64" => DataType::I64,
            "f32" => DataType::F32,
            "f64" => DataType::F64,
            raw if raw.starts_with("raw(") && raw.ends_with(')') => {
                let size = raw.strip_prefix("str(").unwrap().strip_prefix(')').unwrap().parse::<u16>().unwrap_or_else(
                    |_| panic!("Raw `ty` variant has invalid size argument.")
                );
                    DataType::Raw(size)
            },
            s => panic!("Invalid `ty` variant \"{s}\". Use one of \"u16\", \"u32\", \"u64\", \"i16\", \"i32\", \"i64\", \"f32\", \"f64\" or \"raw(size)\"."),
        }
    }
}

impl DataType {
    pub fn word_size(&self) -> Quantity {
        match self {
            DataType::U16 => 1,
            DataType::U32 => 2,
            DataType::U64 => 4,
            DataType::I16 => 1,
            DataType::I32 => 2,
            DataType::I64 => 4,
            DataType::F32 => 2,
            DataType::F64 => 4,
            &DataType::Raw(size) => size,
        }
    }
}

impl From<String> for WordOrder {
    fn from(value: String) -> Self {
        match value.as_str() {
            "be" => WordOrder::BigEndian,
            "le" => WordOrder::LittleEndian,
            s => panic!(
                "Invalid `WordOrder` variant \"{s}\". Use \"be\" for BigEndian or \"le\" for LittleEndian."
            ),
        }
    }
}

impl From<Field> for Entry {
    fn from(value: Field) -> Self {
        let field_name = value
            .ident
            .unwrap_or_else(|| panic!("Unexpected unnamed struct field."))
            .to_string();

        let field_ty = match value.ty {
            Type::Path(type_path) => type_path
                .path
                .get_ident()
                .unwrap_or_else(|| panic!("Unexpected no ident for `{field_name}` field type."))
                .to_string(),
            _ => panic!("Unexpected `syn::Type` variant in `{field_name}` field."),
        };

        let attr = value
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("modbus"))
            .unwrap_or_else(|| {
                panic!("Unexpected missing attribute `modbus` for `{field_name}` field.")
            })
            .clone();
        let punctuated = get_punctuated(&attr, &field_name);

        let addr = Self::get_addr(&punctuated, &field_name);
        let ord = Self::get_ord(&punctuated, &field_name);
        let ty = Self::get_ty(&punctuated, &field_name);
        let x = Self::get_x(&punctuated, &field_name);
        let unit = Self::get_unit(&punctuated, &field_name);

        Self {
            field_name,
            field_ty,
            addr,
            ty,
            ord,
            x,
            unit,
        }
    }
}

impl Entry {
    // Macro helpers

    pub fn fn_to_words(&self) -> Ident {
        match &self.ord {
            WordOrder::BigEndian => Ident::new("to_be_words", Span::call_site()),
            WordOrder::LittleEndian => Ident::new("to_le_words", Span::call_site()),
        }
    }

    pub fn fn_from_words(&self) -> Ident {
        match &self.ord {
            WordOrder::BigEndian => Ident::new("from_be_words", Span::call_site()),
            WordOrder::LittleEndian => Ident::new("from_le_words", Span::call_site()),
        }
    }

    pub fn ty_ident(&self) -> Ident {
        let ty = match &self.ty {
            DataType::U16 => "u16",
            DataType::U32 => "u32",
            DataType::U64 => "u64",
            DataType::I16 => "i16",
            DataType::I32 => "i32",
            DataType::I64 => "i64",
            DataType::F32 => "f32",
            DataType::F64 => "f64",
            DataType::Raw(_size) => todo!(),
        };
        Ident::new(ty, Span::call_site())
    }

    pub fn field_name_ident(&self) -> Ident {
        Ident::new(&self.field_name, Span::call_site())
    }

    pub fn field_ty_ident(&self) -> Ident {
        Ident::new(&self.field_ty, Span::call_site())
    }

    pub fn write_method_ident(&self) -> Ident {
        let name = format!("write_field_{}_to_registers", self.field_name);
        Ident::new(&name, Span::call_site())
    }

    // Parsing helpers

    fn get_addr(punctuated: &Punctuated<ExprAssign, Comma>, field_name: &str) -> Address {
        punctuated
            .iter()
            .filter(expr_assign_predicate("addr", field_name))
            .map(|expr_assign| match *expr_assign.right.clone() {
                Expr::Lit(right) => match right.lit {
                    Lit::Int(lit_int) => lit_int,
                    _ => panic_not_literal("addr", "integer", field_name),
                },
                _ => panic_not_literal("addr", "", field_name),
            })
            .next()
            .unwrap_or_else(|| panic_no_key("addr", field_name))
            .base10_parse::<Address>()
            .unwrap_or_else(|_| panic!("In `modbus` attribute for `{field_name}`, the key `addr` could not be parsed to u16."))
    }

    fn get_ty(punctuated: &Punctuated<ExprAssign, Comma>, field_name: &str) -> DataType {
        punctuated
            .iter()
            .filter(expr_assign_predicate("ty", field_name))
            .map(|expr_assign| match *expr_assign.right.clone() {
                Expr::Lit(right) => match right.lit {
                    Lit::Str(lit_str) => lit_str,
                    _ => panic_not_literal("ty", "string", field_name),
                },
                _ => panic_not_literal("ty", "", field_name),
            })
            .next()
            .unwrap_or_else(|| panic_no_key("ty", field_name))
            .value()
            .into()
    }

    fn get_ord(punctuated: &Punctuated<ExprAssign, Comma>, field_name: &str) -> WordOrder {
        punctuated
            .iter()
            .filter(expr_assign_predicate("ord", field_name))
            .map(|expr_assign| match *expr_assign.right.clone() {
                Expr::Lit(right) => match right.lit {
                    Lit::Str(lit_str) => lit_str,
                    _ => panic_not_literal("ord", "string", field_name),
                },
                _ => panic_not_literal("ord", "", field_name),
            })
            .next()
            .unwrap_or_else(|| panic_no_key("ord", field_name))
            .value()
            .into()
    }

    fn get_x(punctuated: &Punctuated<ExprAssign, Comma>, field_name: &str) -> ScaleFactor {
        punctuated
                    .iter()
                    .filter(expr_assign_predicate("x", field_name))
                    .map(|expr_assign| match *expr_assign.right.clone() {
                        Expr::Lit(right) => match right.lit {
                            Lit::Float(lit_float) => lit_float,
                            _ => panic_not_literal("x", "", field_name)
                        },
                        _ => panic_not_literal("x", "float", field_name)
                    })
                    .next()
                    .unwrap_or_else(|| panic_no_key("x", field_name))
                    .base10_parse::<ScaleFactor>()
                    .unwrap_or_else(|_| panic!("In `modbus` attribute for `{field_name}`, the key `x` could not be parsed to f64."))
    }

    fn get_unit(punctuated: &Punctuated<ExprAssign, Comma>, field_name: &str) -> String {
        punctuated
            .iter()
            .filter(expr_assign_predicate("unit", field_name))
            .map(|expr_assign| match *expr_assign.right.clone() {
                Expr::Lit(right) => match right.lit {
                    Lit::Str(lit_str) => lit_str,
                    _ => panic_not_literal("x", "", field_name),
                },
                _ => panic_not_literal("x", "string", field_name),
            })
            .next()
            .unwrap_or_else(|| panic_no_key("x", field_name))
            .value()
    }
}
