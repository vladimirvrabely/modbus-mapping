use syn::{punctuated::Punctuated, token::Comma, Attribute, Expr, ExprAssign, Meta};

pub fn get_punctuated(attr: &Attribute, name: &str) -> Punctuated<ExprAssign, Comma> {
    match &attr.meta {
                Meta::List(meta_list) => meta_list
                    .clone()
                    .parse_args_with(Punctuated::<ExprAssign, Comma>::parse_terminated)
                    .unwrap_or_else(|_| panic!("`modbus` attribute for `{name}` is not a comma separated sequence of assignment expressions.")),
                _ => panic!("The `modbus` attribute is not `MetaList`"),
            }
}

pub fn expr_assign_predicate<'a>(
    key: &'a str,
    name: &'a str,
) -> impl FnMut(&&ExprAssign) -> bool + 'a {
    move |expr_assign| match *expr_assign.left.clone() {
        Expr::Path(left) => left.path.is_ident(key),
        not_expr_path => panic!(
            "In the `modbus` attribute for {}, the key `{:?}` is not a path expression.",
            name, not_expr_path
        ),
    }
}
pub fn panic_not_literal(key: &str, lit_ty: &str, name: &str) -> ! {
    panic!(
        "In `modbus` attribute for `{}`, the key `{}` is not set to a {} literal.",
        name, key, lit_ty
    )
}

pub fn panic_no_key(key: &str, name: &str) -> ! {
    panic!("In `modbus` attribute for `{}`, no key `{}`", name, key)
}
