use crate::config::Config;
use crate::entry::{Address, Entry, Quantity, ScaleFactor};
use proc_macro2::Ident;
use syn::{Data, DeriveInput, Fields};

#[derive(Debug, Clone)]
pub struct Mapping(pub Vec<Entry>);

impl Mapping {
    pub fn new(ast: &DeriveInput) -> Self {
        let data_struct = match ast.data.clone() {
            Data::Struct(data_struct) => data_struct,
            _ => panic!("Trait can be implemented only for a struct."),
        };

        let named_fields = match data_struct.fields {
            Fields::Named(fields_named) => fields_named.named,
            _ => panic!("Trait can be implemented only for a struct with named fields."),
        };
        let mut map: Vec<Entry> = named_fields
            .into_iter()
            .filter(|field| {
                field
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("modbus"))
            })
            .map(From::from)
            .collect::<Vec<_>>();

        map.sort_by_key(|x| x.addr);

        Self(map)
    }

    pub fn field_name_vec(&self) -> Vec<Ident> {
        self.0
            .iter()
            .map(|x| x.field_name_ident())
            .collect::<Vec<_>>()
    }

    pub fn field_ty_vec(&self) -> Vec<Ident> {
        self.0
            .iter()
            .map(|x| x.field_ty_ident())
            .collect::<Vec<_>>()
    }

    pub fn addr_vec(&self) -> Vec<Address> {
        self.0.iter().map(|x| x.addr).collect::<Vec<_>>()
    }

    pub fn ty_vec(&self) -> Vec<Ident> {
        self.0
            .iter()
            .map(|entry| entry.ty_ident())
            .collect::<Vec<_>>()
    }

    pub fn cnt_vec(&self) -> Vec<Quantity> {
        self.0.iter().map(|x| x.ty.word_size()).collect::<Vec<_>>()
    }

    pub fn fn_to_words_vec(&self) -> Vec<Ident> {
        self.0
            .iter()
            .map(|entry| entry.fn_to_words())
            .collect::<Vec<_>>()
    }

    pub fn fn_from_words_vec(&self) -> Vec<Ident> {
        self.0
            .iter()
            .map(|entry| entry.fn_from_words())
            .collect::<Vec<_>>()
    }

    pub fn x_vec(&self) -> Vec<ScaleFactor> {
        self.0.iter().map(|x| x.x).collect::<Vec<_>>()
    }

    pub fn register_range(&self) -> (Address, Address) {
        if self.0.is_empty() {
            (0, 0)
        } else {
            let first = self.0.first().unwrap();
            let last = self.0.last().unwrap();
            (first.addr, last.addr + last.ty.word_size())
        }
    }
}

impl Mapping {
    pub fn split_into_block_mappings(self, config: &Config) -> Vec<Self> {
        let mut block_mappings = vec![];

        let mut entries = Vec::with_capacity(self.0.len());

        for entry in self.0 {
            if entries.is_empty() {
                entries.push(entry);
            } else {
                let first = entries.first().unwrap();
                let last = entries.last().unwrap();
                let max_cond =
                    entry.addr + entry.ty.word_size() - first.addr <= config.max_cnt_per_request;
                let gap_cond = last.addr + last.ty.word_size() == entry.addr;

                if max_cond && (gap_cond || config.allow_register_gaps) {
                    entries.push(entry)
                } else {
                    let mapping = Mapping(entries.clone());
                    block_mappings.push(mapping);
                    entries.clear();
                    entries.push(entry);
                }
            }
        }

        let mapping = Mapping(entries.clone());
        block_mappings.push(mapping);

        block_mappings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{DataType, WordOrder};

    #[test]
    fn test_split_into_block_mappings() {
        let addrs = [0, 4, 8, 10];
        let mapping = Mapping(
            addrs
                .into_iter()
                .map(|addr| Entry {
                    field_name: format!("field_{}", addr),
                    field_ty: format!("ty_{}", addr),
                    addr: addr,
                    ty: DataType::F32,
                    ord: WordOrder::BigEndian,
                    x: 1.0,
                    unit: format!("unit_{}", addr),
                })
                .collect(),
        );

        for (allow_register_gaps, expected) in [(true, vec![2, 2]), (false, vec![1, 1, 2])] {
            let config = Config {
                max_cnt_per_request: 8,
                allow_register_gaps,
            };
            let result = mapping
                .clone()
                .split_into_block_mappings(&config)
                .iter()
                .map(|m| m.0.len())
                .collect::<Vec<_>>();
            assert_eq!(result, expected)
        }
    }
}
