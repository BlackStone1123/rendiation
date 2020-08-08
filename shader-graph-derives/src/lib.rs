use proc_macro::TokenStream;

use syn::parse_macro_input;

mod bindgroup;
mod geometry;
mod glsl_fn;
mod ubo;
mod utils;
use bindgroup::*;
use geometry::*;
use glsl_fn::*;
use ubo::*;

#[proc_macro_derive(UniformBuffer)]
pub fn derive_ubo(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_ubo_impl(&input)
    .unwrap_or_else(|err| err.to_compile_error())
    .into()
}

#[proc_macro_derive(BindGroup)]
pub fn derive_bindgroup(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_bindgroup_impl(input)
    .unwrap_or_else(|err| err.to_compile_error())
    .into()
}

#[proc_macro_derive(Geometry)]
pub fn derive_geometry(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::DeriveInput);
  derive_geometry_impl(input)
    .unwrap_or_else(|err| err.to_compile_error())
    .into()
}

#[proc_macro]
pub fn glsl_function(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as syn::LitStr);
  let glsl = input.value();
  gen_glsl_function(&glsl).into()
}
