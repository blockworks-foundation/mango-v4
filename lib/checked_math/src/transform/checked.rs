use proc_macro_error::abort;
use quote::quote;
use syn::{spanned::Spanned, BinOp, Expr, ExprBinary, ExprUnary, Ident, Lit, UnOp};

pub fn transform_expr(mut expr: Expr) -> proc_macro2::TokenStream {
    match expr {
        Expr::Unary(unary) => transform_unary(unary),
        Expr::Binary(binary) => transform_binary(binary),
        Expr::MethodCall(ref mut mc) => {
            if mc.method == "pow" {
                mc.method = syn::Ident::new("checked_pow", mc.method.span());
                quote! { #mc }
            } else if mc.method == "abs" {
                mc.method = syn::Ident::new("checked_abs", mc.method.span());
                quote! { #mc }
            } else if mc.args.is_empty() {
                quote! { Some(#mc) }
            } else {
                abort!(mc, "method calls with arguments are not supported");
            }
        }
        Expr::Call(ref mut c) => {
            if c.args.is_empty() {
                quote! { Some(#c) }
            } else {
                abort!(c, "calls with arguments are not supported");
            }
        }
        Expr::Paren(p) => {
            let expr = transform_expr(*p.expr);
            quote! {
                (#expr)
            }
        }
        Expr::Group(g) => {
            let expr = transform_expr(*g.expr);
            quote! {
                (#expr)
            }
        }
        Expr::Lit(lit) => match lit.lit {
            Lit::Int(_) | Lit::Float(_) => quote! { Some(#lit) },
            _ => abort!(lit, "unsupported literal"),
        },
        Expr::Path(_) | Expr::Field(_) => {
            quote! { Some(#expr) }
        }
        _ => {
            abort!(expr, "unsupported expr {:?}", expr);
        }
    }
}

fn transform_unary(unary: ExprUnary) -> proc_macro2::TokenStream {
    let expr = transform_expr(*unary.expr);
    let op = unary.op;
    match op {
        UnOp::Neg(_) => {
            quote! {
                {
                    match #expr {
                        Some(e) => e.checked_neg(),
                        None => None
                    }
                }
            }
        }
        UnOp::Deref(_) => quote! { #expr },
        UnOp::Not(_) => abort!(expr, "unsupported unary expr"),
    }
}

fn transform_binary(binary: ExprBinary) -> proc_macro2::TokenStream {
    let left = transform_expr(*binary.left);
    let right = transform_expr(*binary.right);
    let op = binary.op;
    let method_name = match op {
        BinOp::Add(_) => Some("checked_add"),
        BinOp::Sub(_) => Some("checked_sub"),
        BinOp::Mul(_) => Some("checked_mul"),
        BinOp::Div(_) => Some("checked_div"),
        BinOp::Rem(_) => Some("checked_rem"),
        BinOp::Shl(_) => Some("checked_shl"),
        BinOp::Shr(_) => Some("checked_shr"),
        _ => abort!(op, "unsupported binary expr"),
    };
    method_name
        .map(|method_name| {
            let method_name = Ident::new(method_name, op.span());
            quote! {
                {
                    match (#left, #right) {
                        (Some(left), Some(right)) => left.#method_name(right),
                        _ => None
                    }

                }
            }
        })
        .unwrap_or_else(|| {
            quote! {
                match (#left, #right) {
                    (Some(left), Some(right)) => left #op right,
                    _ => None
                }
            }
        })
}
