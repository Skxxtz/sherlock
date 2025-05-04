use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, LitStr};

#[proc_macro_attribute]
pub fn timing(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_body = &input.block;
    let fn_vis = &input.vis;
    let fn_sig = &input.sig;

    let user_str = if attr.is_empty() {
        format!(r#"Function "{}""#, fn_name)
    } else {
        let attr_str = parse_macro_input!(attr as LitStr);
        attr_str.value()
    };

    let expanded = quote! {
        #fn_vis #fn_sig {
            if let Ok(timing_enabled) = std::env::var("TIMING") {
                if timing_enabled == "true" {
                    let start = std::time::Instant::now();
                    let result = (|| #fn_body)();
                    let duration = start.elapsed();
                    println!("{} took {:?}", #user_str, duration);
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
