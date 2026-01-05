use proc_macro::TokenStream;
use quote::quote;
use syn::Expr::Lit;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{ItemFn, Meta, parse_macro_input};

#[proc_macro_attribute]
pub fn timing(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let args: Punctuated<Meta, Comma> = parse_macro_input!(attr with Punctuated::parse_terminated);
    let fn_name = &input.sig.ident;
    let fn_body = &input.block;
    let fn_vis = &input.vis;
    let fn_sig = &input.sig;

    let mut name = format!("Function \"{}\"", fn_name);
    let mut level = "true".to_string();
    for arg in args {
        if let Meta::NameValue(nv) = arg {
            let ident = nv.path.get_ident().map(|i| i.to_string());
            if let Some(ident) = ident {
                match (&ident[..], &nv.value) {
                    ("name", Lit(expr_lit)) => {
                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                            name = lit_str.value();
                        }
                    }
                    ("level", Lit(expr_lit)) => {
                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                            level = lit_str.value();
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let expanded = quote! {
        #fn_vis #fn_sig {
            if let Ok(timing_enabled) = std::env::var("TIMING") {
                if timing_enabled == "all" || timing_enabled == #level {
                    let start = std::time::Instant::now();
                    let result = (|| #fn_body)();
                    let duration = start.elapsed();
                    println!("{} took {:?}", #name, duration);
                    result
                } else {
                    (|| #fn_body)()
                }
            } else {
                (|| #fn_body)()
            }
        }
    };

    TokenStream::from(expanded)
}
