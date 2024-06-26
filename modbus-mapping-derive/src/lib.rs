//! Macros to `derive` the `modbus-mapping` traits

use proc_macro::TokenStream;
use quote::quote;

mod config;
mod entry;
mod mapping;
mod utils;

/// Derive macro to implement `modbus_mapping::core::InputRegisterMap`
#[proc_macro_derive(InputRegisterMap, attributes(modbus))]
pub fn derive_input_register_map(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);

    let name = &ast.ident;

    let block_config = config::Config::new(&ast);

    let mapping = mapping::Mapping::new(&ast);
    let block_mappings = mapping.split_into_block_mappings(&block_config);

    let mut blocks = Vec::new();
    for mapping in block_mappings {
        let field_name = mapping.field_name_vec();
        let field_ty = mapping.field_ty_vec();
        let x = mapping.x_vec();
        let addr = mapping.addr_vec();
        let ty = mapping.ty_vec();
        let cnt = mapping.cnt_vec();
        let from_words = mapping.fn_from_words_vec();

        let (start, end) = mapping.register_range();
        let len = end - start;

        let block = quote! {
            // Read
            let words = match client.read_input_registers(#start, #len).await? {
                Ok(words) => words,
                Err(exc) => return Ok(Err(exc)),
            };
            #(
                // Decode
                let #field_name: #ty = modbus_mapping::codec::Decode::#from_words(&words[(#addr - #start) as usize..(#addr - #start + #cnt) as usize]).unwrap();
                // Convert and scale
                #[allow(clippy::unnecessary_cast)]
                let #field_name: #field_ty = (#field_name as #field_ty) * (#x as #field_ty);
                // Set
                self.#field_name = #field_name;

            )*

        };
        blocks.push(block);
    }

    let tokens = quote! {
        #[async_trait::async_trait]
        impl modbus_mapping::core::InputRegisterMap for #name {
            async fn update_from_input_registers(&mut self, client: &mut dyn tokio_modbus::client::Reader) -> tokio_modbus::Result<()>{
                #(#blocks)*
                Ok(Ok(()))
            }
        }

    };

    tokens.into()
}

/// Derive macro to implement `modbus_mapping::core::HoldingRegisterMap`
#[proc_macro_derive(HoldingRegisterMap, attributes(modbus))]
pub fn derive_holding_register_map(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);

    let name = &ast.ident;

    let block_config = config::Config::new(&ast);

    let mapping = mapping::Mapping::new(&ast);
    let block_mappings = mapping.clone().split_into_block_mappings(&block_config);

    let mut read_blocks = Vec::new();
    for mapping in block_mappings {
        let field_name = mapping.field_name_vec();
        let field_ty = mapping.field_ty_vec();
        let x = mapping.x_vec();
        let addr = mapping.addr_vec();
        let ty = mapping.ty_vec();
        let cnt = mapping.cnt_vec();
        let from_words = mapping.fn_from_words_vec();

        let (start, end) = mapping.register_range();
        let len = end - start;

        let block = quote! {
            // Read
            let words = match client.read_holding_registers(#start, #len).await? {
                Ok(words) => words,
                Err(exc) => return Ok(Err(exc)),
            };
            #(
                // Decode
                let #field_name: #ty = modbus_mapping::codec::Decode::#from_words(&words[(#addr - #start) as usize..(#addr - #start + #cnt) as usize]).unwrap();
                // Convert and scale
                #[allow(clippy::unnecessary_cast)]
                let #field_name: #field_ty = (#field_name as #field_ty) * (#x as #field_ty);
                // Set
                self.#field_name = #field_name;

            )*

        };
        read_blocks.push(block);
    }

    let mut write_blocks = Vec::new();
    let mut method_blocks = write_blocks.clone();
    for entry in mapping.0 {
        let field_name = entry.field_name_ident();
        let field_ty = entry.field_ty_ident();
        let x = &entry.x;
        let addr = &entry.addr;
        let ty = entry.ty_ident();
        let to_words = entry.fn_to_words();

        let mut block = quote! {
            // Convert and rescale
            #[allow(clippy::unnecessary_cast)]
            let #field_name: #ty = (self.#field_name / (#x as #field_ty)) as #ty;
            // Encode
            let field_words: Vec<modbus_mapping::codec::Word> = modbus_mapping::codec::Encode::#to_words(#field_name);
        };
        if entry.ty.word_size() == 1 {
            block.extend(quote! {
                match client.write_single_register(#addr, field_words.first().unwrap().clone()).await? {
                    Ok(_) => {},
                    Err(exc) => return Ok(Err(exc))
                };
            })
        } else {
            block.extend(quote! {
                // Set (use splice alternatively)
                match client.write_multiple_registers(#addr, &field_words).await? {
                    Ok(_) => {},
                    Err(exc) => return Ok(Err(exc))
                };
            })
        }

        write_blocks.push(block.clone());

        let method = entry.write_method_ident();
        let block = quote! {
            pub async fn #method(&self, client: &mut dyn tokio_modbus::client::Writer) -> tokio_modbus::Result<()> {
                #block
                Ok(Ok(()))
            }
        };
        method_blocks.push(block.clone());
    }

    let tokens = quote! {
        #[async_trait::async_trait]
        impl modbus_mapping::core::HoldingRegisterMap for #name {
            async fn update_from_holding_registers(&mut self, client: &mut dyn tokio_modbus::client::Reader) -> tokio_modbus::Result<()>{
                // #(#read_blocks)*
                Ok(Ok(()))
            }

            async fn write_to_registers(&self, client: &mut dyn tokio_modbus::client::Writer) -> tokio_modbus::Result<()> {
                #(#write_blocks)*;
                Ok(Ok(()))
            }
        }

        impl #name {
            #(#method_blocks)*
        }

    };

    tokens.into()
}

#[proc_macro_attribute]
pub fn modbus_doc(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut ast = syn::parse_macro_input!(item as syn::DeriveInput);
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {
            match &mut struct_data.fields {
                syn::Fields::Named(fields_named) => {
                    for field in &mut fields_named.named {
                        if field
                            .attrs
                            .iter()
                            .any(|attr| attr.path().is_ident("modbus"))
                        {
                            let entry: entry::Entry = field.clone().into();
                            let doc = format!("address - `{}`, data type - `{:?}` (`{}` registers), word order - `{:?}`, scale factor - `{}`, unit - `{}`.", entry.addr, entry.ty, entry.ty.word_size(), entry.ord, entry.x, entry.unit);
                            let doc: syn::Attribute = syn::parse_quote!(#[doc = #doc]);
                            field.attrs.push(doc);
                        }
                    }
                }
                _ => panic!("`modbus_doc` has to be applied to structs with named fields"),
            }

            quote! {
                #ast
            }
            .into()
        }
        _ => panic!("`modbus_doc` has to be applied with structs"),
    }
}

/// Derive macro to implement `modbus_mapping::core::InputRegisterModel`
#[proc_macro_derive(InputRegisterModel, attributes(modbus))]
pub fn derive_input_register_model(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);

    let name = &ast.ident;

    let mapping = mapping::Mapping::new(&ast);
    // panic!("{:#?}", mapping);

    let field_name = mapping.field_name_vec();
    let field_ty = mapping.field_ty_vec();
    let x = mapping.x_vec();
    let addr = mapping.addr_vec();
    let ty = mapping.ty_vec();
    let to_words = mapping.fn_to_words_vec();

    let tokens = quote! {
        impl modbus_mapping::simulator::InputRegisterModel for #name {
            fn new_registers(&self) -> modbus_mapping::simulator::Registers {
                let mut registers = modbus_mapping::simulator::Registers::default();

                #(
                    // Divide by scale factor
                    #[allow(clippy::unnecessary_cast)]
                    let #field_name: #ty = (&self.#field_name / (#x as #field_ty)) as #ty;
                    registers.insert(#addr, modbus_mapping::codec::Encode::#to_words(#field_name));
                )*

                registers
            }

            fn update_registers(
                &self,
                registers: &mut modbus_mapping::simulator::Registers,
            ) -> Result<(), tokio_modbus::Exception> {
                #(
                    // Divide by scale factor
                    #[allow(clippy::unnecessary_cast)]
                    let #field_name: #ty = (self.#field_name / (#x as #field_ty)) as #ty;
                    registers.write(#addr, &modbus_mapping::codec::Encode::#to_words(#field_name))?;
                )*
                Ok(())
            }
        }
    };

    tokens.into()
}

/// Derive macro to implement `modbus_mapping::core::HoldingRegisterModel`
#[proc_macro_derive(HoldingRegisterModel, attributes(modbus))]
pub fn derive_holding_register_model(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);

    let name = &ast.ident;

    let mapping = mapping::Mapping::new(&ast);
    // panic!("{:#?}", mapping);

    let field_name = mapping.field_name_vec();
    let field_ty = mapping.field_ty_vec();
    let x = mapping.x_vec();
    let addr = mapping.addr_vec();
    let cnt = mapping.cnt_vec();
    let ty = mapping.ty_vec();
    let to_words = mapping.fn_to_words_vec();
    let from_words = mapping.fn_from_words_vec();

    let tokens = quote! {
        impl modbus_mapping::simulator::HoldingRegisterModel for #name {
            fn new_registers(&self) -> modbus_mapping::simulator::Registers {
                let mut registers = modbus_mapping::simulator::Registers::default();

                #(
                    // Divide by scale factor
                    #[allow(clippy::unnecessary_cast)]
                    let #field_name: #ty = (&self.#field_name / (#x as #field_ty)) as #ty;
                    registers.insert(#addr, modbus_mapping::codec::Encode::#to_words(#field_name));
                )*

                registers
            }

            fn update_registers(
                &self,
                registers: &mut modbus_mapping::simulator::Registers,
            ) -> Result<(), tokio_modbus::Exception> {
                #(
                    // Divide by scale factor
                    #[allow(clippy::unnecessary_cast)]
                    let #field_name: #ty = (self.#field_name / (#x as #field_ty)) as #ty;
                    registers.write(#addr, &modbus_mapping::codec::Encode::#to_words(#field_name))?;

                )*

                Ok(())
            }

            fn update_self(&mut self, registers: &modbus_mapping::simulator::Registers) -> Result<(), tokio_modbus::Exception> {
                #(
                    // Read
                    let words = registers.read(#addr, #cnt)?;
                    // Decode
                    let #field_name: #ty = modbus_mapping::codec::Decode::#from_words(&words).unwrap();
                    // Convert and scale
                    #[allow(clippy::unnecessary_cast)]
                    let #field_name: #field_ty = (#field_name as #field_ty) * (#x as #field_ty);
                    // Set
                    self.#field_name = #field_name;
                )*

                Ok(())
            }
        }

    };

    tokens.into()
}
