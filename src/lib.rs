use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn;

/// Derives the CompileConst trait for structs and enums. Requires that all
/// fields also implement the CompileConst trait.
#[proc_macro_derive(CompileConst)]
pub fn const_gen_derive(input: TokenStream) -> TokenStream 
{
    impl_macro(&syn::parse(input).unwrap())
}

fn impl_macro(ast: &syn::DeriveInput) -> TokenStream 
{
    let name = &ast.ident;
    let generics = &ast.generics;
    let val_impl: proc_macro2::TokenStream = match &ast.data
    {
        syn::Data::Struct(data) => struct_val_handler(name, &data.fields),
        syn::Data::Enum(data) => 
        {
            let arms: Vec<proc_macro2::TokenStream> = data.variants
                .iter()
                .map(|v| enum_val_handler(name, &v.ident, &v.fields))
                .collect();
            quote!
            {
                format!("{}", match self
                {
                    #( #arms, )*
                })
            }
        }
        syn::Data::Union(data) => 
        {
            let vis = get_field_visibilities(&data.fields.named).into_iter().next().unwrap();
            let ident = get_field_idents(&data.fields.named).into_iter().next().unwrap();
            quote!
            {
                format!
                (
                    "{} {} {{ {}: {}}}", 
                    stringify!(#vis), 
                    stringify!(#name), 
                    stringify!(#ident), 
                    self.#ident.const_val()
                )
            }
        }
    };
    let def_impl: proc_macro2::TokenStream = match &ast.data
    {
        syn::Data::Struct(data) => struct_def_handler(name, generics, &data.fields),
        syn::Data::Enum(data) => enum_def_handler(name, generics, data.variants.iter().collect()),
        syn::Data::Union(data) => 
        {
            let vis = get_field_visibilities(&data.fields.named);
            let idents = get_field_idents(&data.fields.named);
            let types = get_field_types(&data.fields.named);
            quote!
            {
                let mut f = String::new();
                #( f.push_str(&format!("{} {}: {}, ", stringify!(#vis), stringify!(#idents), <#types>::const_type())); )*
                format!
                (
                    "union {}{}{{ {}}}", 
                    stringify!(#name), 
                    stringify!(#generics), 
                    f
                )
            }
        }
    };
    let gen = quote!
    {
        impl const_gen::CompileConst for #name #generics
        {
            fn const_type() -> String
            {
                String::from(stringify!(#name))
            }

            fn const_val(&self) -> String
            {
                #val_impl
            }

            fn const_definition(attrs: &str, vis: &str) -> String
            {
                let mut definition = String::from(attrs);
                definition += " ";
                definition += vis;
                definition += &{#def_impl};
                definition
            }
        }
    };
    gen.into()
}

/// Generate a struct definition
fn struct_def_handler(name: &syn::Ident, generics: &syn::Generics, fields: &syn::Fields) -> proc_macro2::TokenStream
{
    match fields
    {
        syn::Fields::Named(f) => 
        {
            let vis = get_field_visibilities(&f.named);
            let idents = get_field_idents(&f.named);
            let types = get_field_types(&f.named);
            quote!
            {
                let mut f = String::new();
                #( f.push_str(&format!("{} {}: {}, ", stringify!(#vis), stringify!(#idents), <#types>::const_type())); )*
                format!
                (
                    "struct {}{}{{ {}}}", 
                    stringify!(#name), 
                    stringify!(#generics), 
                    f
                )
            }
        },
        syn::Fields::Unnamed(f) => 
        {
            let types = get_field_types(&f.unnamed);
            quote!
            {
                let mut f = String::new();
                #( f.push_str(&format!("{},", <#types>::const_type())); )*
                format!
                (
                    "struct {}{}({});", 
                    stringify!(#name), 
                    stringify!(#generics), 
                    f
                )
            }
        },
        syn::Fields::Unit => quote!(format!("struct {}{};", stringify!(#name), stringify!(#generics)))
    } 
}

/// Generate a struct constructor
fn struct_val_handler(name: &syn::Ident, fields: &syn::Fields) -> proc_macro2::TokenStream
{
    match fields
    {
        syn::Fields::Named(f) => 
        {
            let idents = get_field_idents(&f.named);
            quote!
            {
                let mut f = String::new();
                #( f.push_str(&format!("{}: {}, ", stringify!(#idents), self.#idents.const_val())); )*
                format!
                (
                    "{} {{ {}}}", 
                    stringify!(#name), 
                    f
                )
            }
        },
        syn::Fields::Unnamed(f) => 
        {
            let mut counter = 0;
            let vals: Vec<_> = f.unnamed.iter()
                .map(|_|{let next = counter; counter += 1; next})
                .map(syn::Index::from).collect();
            quote!
            {
                let mut f = String::new();
                #( f.push_str(&format!("{},", self.#vals.const_val())); )*
                format!
                (
                    "{}({})", 
                    stringify!(#name), 
                    f
                )
            }
        },
        syn::Fields::Unit => quote!(stringify!(#name))
    } 
}

/// Generate an enum constructor
fn enum_val_handler(name: &syn::Ident, var_name: &syn::Ident, fields: &syn::Fields) -> proc_macro2::TokenStream
{
    let constructor = match fields
    {
        syn::Fields::Named(f) => 
        {
            let idents = get_field_idents(&f.named);
            quote!
            {{
                let mut f = String::new();
                #( f.push_str(&format!("{}:{},", stringify!(#idents), #idents.const_val())); )*
                format!
                (
                    "{}::{}{{{}}}", 
                    stringify!(#name), 
                    stringify!(#var_name), 
                    f
                )
            }}
        },
        syn::Fields::Unnamed(f) => 
        {
            let mut counter = 0;
            let idents: Vec<syn::Ident> = f.unnamed.iter()
                .map(|_|
                {
                    let new_ident = syn::Ident::new(&format!("idnt{}", counter), Span::call_site());
                    counter += 1;
                    new_ident
                })
                .collect();
            quote!
            {{
                let mut f = String::new();
                #( f.push_str(&format!("{},", #idents.const_val())); )*
                format!
                (
                    "{}::{}({})", 
                    stringify!(#name), 
                    stringify!(#var_name), 
                    f
                )
            }}
        },
        syn::Fields::Unit => quote!(format!("{}::{}", stringify!(#name), stringify!(#var_name)))
    };
    match fields
    {
        syn::Fields::Named(f) => 
        {
            let new_idents = get_field_idents(&f.named);
            quote!(Self::#var_name {#(#new_idents, )*} => #constructor)
        },
        syn::Fields::Unnamed(f) => 
        {
            let mut counter = 0;
            let new_idents: Vec<syn::Ident> = f.unnamed.iter()
                .map(|_|
                {
                    let new_ident = syn::Ident::new(&format!("idnt{}", counter), Span::call_site());
                    counter += 1;
                    new_ident
                })
                .collect();
            quote!(Self::#var_name (#(#new_idents, )*) => #constructor)
        }
        syn::Fields::Unit => quote!(Self::#var_name => #constructor)
    }
}

/// Generate an enum definition
fn enum_def_handler(name: &syn::Ident, generics: &syn::Generics, variants: Vec<&syn::Variant>) -> proc_macro2::TokenStream
{
    let arms: Vec<proc_macro2::TokenStream> = variants.into_iter()
        .map(|v| enum_variant_def_handler(&v.ident, &v.fields))
        .collect();
    quote!
    {
        format!("enum {}{}{{ {} }}", stringify!(#name), stringify!(#generics), #(#arms+)&* "")
    }
}

/// Generate an enum variant definition
fn enum_variant_def_handler(var_name: &syn::Ident, fields: &syn::Fields) -> proc_macro2::TokenStream
{
    match fields
    {
        syn::Fields::Named(f) => 
        {
            let idents = get_field_idents(&f.named);
            let types = get_field_types(&f.named);
            quote!
            {{
                let mut f = String::new();
                #( f.push_str(&format!("{}:{},", stringify!(#idents), <#types>::const_type())); )*
                format!
                (
                    "{}{{{}}},", 
                    stringify!(#var_name), 
                    f
                )
            }}
        },
        syn::Fields::Unnamed(f) => 
        {
            let types = get_field_types(&f.unnamed);
            quote!
            {{
                let mut f = String::new();
                #( f.push_str(&format!("{},", <#types>::const_type())); )*
                format!
                (
                    "{}({}),", 
                    stringify!(#var_name), 
                    f
                )
            }}
        },
        syn::Fields::Unit => quote!(format!("{},", stringify!(#var_name)))
    }
}

/// Get visibility of named fields
fn get_field_visibilities(fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>) -> Vec<&syn::Visibility>
{
    fields.iter().map(|field|&field.vis).collect()
}

/// Get identifiers of named fields
fn get_field_idents(fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>) -> Vec<&Option<syn::Ident>>
{
    fields.iter().map(|field|&field.ident).collect()
}

/// Get types of fields
fn get_field_types(fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>) -> Vec<&syn::Type>
{
    fields.iter().map(|field|&field.ty).collect()
}
