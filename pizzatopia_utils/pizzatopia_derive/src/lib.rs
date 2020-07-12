extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_attribute]
pub fn enum_cycle(_metadata: TokenStream, input: TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let output = quote! {
        #[derive(EnumIter, EnumCount, EnumCycle)]
        #input
    };
    output.into()
}

// Create the derive macro
#[proc_macro_derive(EnumCycle)]
pub fn enum_cycle_macro(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
    impl From<usize> for #name {
        fn from(x: usize) -> Self {
            Self::iter().nth(x).unwrap()
        }
    }
    impl EnumCycle for #name {
        fn next(&self) -> Self {
            let mut index = (*self as usize) + 1;
            if index >= Self::iter().count() {
                index = 0;
            }

            return Self::from(index);
        }

        fn prev(&self) -> Self {
            let mut index = *self as usize;
            if index <= 0 {
                index = Self::iter().count();
            }

            index -= 1;
            return Self::from(index);
        }
    }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}
