use proc_macro::TokenStream;
use quote::{quote,format_ident};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Hello)]
pub fn hello_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let fn_name = format_ident!("say_hello_{}", name);

    quote!{
        //#[verifier::external_body]
        fn #fn_name() {
            println!("#name says hi");
        }
    }.into()
}
