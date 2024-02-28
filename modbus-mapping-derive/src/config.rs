use crate::entry::Quantity;
use crate::utils::{expr_assign_predicate, get_punctuated, panic_not_literal};
use syn::{punctuated::Punctuated, token::Comma, DeriveInput, Expr, ExprAssign, Lit};

#[derive(Debug)]
pub struct Config {
    pub max_cnt_per_request: Quantity,
    pub allow_register_gaps: bool,
}

impl Config {
    pub fn new(ast: &DeriveInput) -> Self {
        let attr = &ast.attrs.iter().find(|attr| attr.path().is_ident("modbus"));

        let name = &ast.ident.to_string();

        // Try to extract relevant fields from the attribute
        let (max_cnt_per_request, allow_register_gaps) = match attr {
            Some(attr) => {
                let punctuated = get_punctuated(attr, name);

                let max_cnt_per_request = Self::get_max_cnt_per_request(&punctuated, name);
                let allow_register_gaps = Self::get_allow_register_gaps(&punctuated, name);

                (max_cnt_per_request, allow_register_gaps)
            }
            None => (None, None),
        };

        Self {
            // https://en.wikipedia.org/wiki/Modbus#Function_codes_4_(read_input_registers)_and_3_(read_holding_registers)
            max_cnt_per_request: max_cnt_per_request.unwrap_or(123),
            allow_register_gaps: allow_register_gaps.unwrap_or(false),
        }
    }

    fn get_max_cnt_per_request(
        punctuated: &Punctuated<ExprAssign, Comma>,
        name: &str,
    ) -> Option<Quantity> {
        punctuated
            .iter()
            .filter(expr_assign_predicate("max_cnt_per_request", name))
            .map(|expr_assign| match *expr_assign.right.clone() {
                Expr::Lit(right) => match right.lit {
                    Lit::Int(lit_int) => lit_int,
                    _ => panic_not_literal("max_cnt_per_request", "integer", name),
                },
                _ => panic_not_literal("max_cnt_per_request", "", name),
            })
            .next()
            .map(|lit_int| lit_int.base10_parse::<Quantity>().unwrap_or_else(|_| panic!("In `modbus` attribute for `{name}`, the key `addr` could not be parsed to u16.")))
    }

    fn get_allow_register_gaps(
        punctuated: &Punctuated<ExprAssign, Comma>,
        name: &str,
    ) -> Option<bool> {
        punctuated
            .iter()
            .filter(expr_assign_predicate("allow_gaps", name))
            .map(|expr_assign| match *expr_assign.right.clone() {
                Expr::Lit(right) => match right.lit {
                    Lit::Bool(lit_bool) => lit_bool,
                    _ => panic_not_literal("allow_gaps", "bool", name),
                },
                _ => panic_not_literal("allow_gaps", "", name),
            })
            .next()
            .map(|lit_bool| lit_bool.value())
    }
}
