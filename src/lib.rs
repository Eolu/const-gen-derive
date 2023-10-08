use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn;

const DOC_ATTR: &'static str = "inherit_doc";
const INNER_DOC_ATTR: &'static str = "inherit_docs";

/// Derives the CompileConst trait for structs and enums. Requires that all
/// fields also implement the CompileConst trait.
#[proc_macro_derive(CompileConst, attributes(inherit_doc, inherit_docs))]
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
    let (doc_attr, inner_doc_attr) = get_docs(&ast.attrs);
    let def_impl: proc_macro2::TokenStream = match &ast.data
    {
        syn::Data::Struct(data) => struct_def_handler(name, generics, &data.fields, inner_doc_attr),
        syn::Data::Enum(data) => enum_def_handler(name, generics, data.variants.iter().collect(), inner_doc_attr),
        syn::Data::Union(data) => 
        {
            let docs = get_field_docs(&data.fields.named, inner_doc_attr);
            let vis = get_field_visibilities(&data.fields.named);
            let idents = get_field_idents(&data.fields.named);
            let types = get_field_types(&data.fields.named);
            quote!
            {
                let mut f = String::new();
                #( f.push_str(&format!("{} {} {}: {}, ", stringify!(#docs), stringify!(#vis), stringify!(#idents), <#types>::const_type())); )*
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
    let doc_attr = doc_attr.map_or(String::new(), |attr| quote!(#attr).to_string());
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
                definition += #doc_attr;
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
fn struct_def_handler(name: &syn::Ident, generics: &syn::Generics, fields: &syn::Fields, inner_doc_attr: bool) -> proc_macro2::TokenStream
{
    match fields
    {
        syn::Fields::Named(f) => 
        {
            let vis = get_field_visibilities(&f.named);
            let idents = get_field_idents(&f.named);
            let types = get_field_types(&f.named);
            let docs = get_field_docs(&f.named, inner_doc_attr);
            quote!
            {
                let mut f = String::new();
                #( f.push_str(&format!("{} {} {}: {}, ", stringify!(#docs), stringify!(#vis), stringify!(#idents), <#types>::const_type())); )*
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
fn enum_def_handler(name: &syn::Ident, generics: &syn::Generics, variants: Vec<&syn::Variant>, inner_doc_attr: bool) -> proc_macro2::TokenStream
{
    let arms: Vec<proc_macro2::TokenStream> = variants.into_iter()
        .map(|v| enum_variant_def_handler(&v.attrs, &v.ident, &v.fields, inner_doc_attr))
        .collect();
    quote!
    {
        format!("enum {}{}{{ {} }}", stringify!(#name), stringify!(#generics), #(#arms+)&* "")
    }
}

/// Generate an enum variant definition
fn enum_variant_def_handler(attributes: &[syn::Attribute], var_name: &syn::Ident, fields: &syn::Fields, inner_doc_attr: bool) -> proc_macro2::TokenStream
{
    let doc_attr = get_inner_docs(attributes, inner_doc_attr);
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
                    "{} {}{{{}}},", 
                    stringify!(#doc_attr),
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
                    "{} {}({}),", 
                    stringify!(#doc_attr),
                    stringify!(#var_name), 
                    f
                )
            }}
        },
        syn::Fields::Unit => quote!(format!("{} {},", stringify!(#doc_attr), stringify!(#var_name)))
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

/// Get docs for fields
fn get_field_docs(fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>, inner_doc_attr: bool) -> Vec<Option<syn::Attribute>>
{
    fields.iter().map(|field| get_inner_docs(&field.attrs, inner_doc_attr)).collect()
}

/// Parse DOC_ATTR to inherit docs
fn get_docs(attrs: &[syn::Attribute]) -> (Option<syn::Attribute>, bool)
{
    let mut inner_doc_attr = false;
    if attrs
        .iter()
        .any(|assoc_attr| 
        {
            if assoc_attr.path.is_ident(INNER_DOC_ATTR)
            {
                inner_doc_attr = true;
            }
            assoc_attr.path.is_ident(DOC_ATTR) || inner_doc_attr
        })
    {
        (attrs.iter()
            .filter(|assoc_attr| assoc_attr.path.is_ident("doc"))
            .next()
            .map(syn::Attribute::clone), inner_doc_attr)
    }
    else
    {
        (None, inner_doc_attr)
    }
}

/// Parse INNER_DOC_ATTR to inherit docs for fields or variants
fn get_inner_docs(attrs: &[syn::Attribute], inner_doc_attr: bool) -> Option<syn::Attribute>
{
    attrs.iter()
        .filter(|assoc_attr| assoc_attr.path.is_ident("doc"))
        .next()
        .filter(|_| inner_doc_attr || attrs.iter().any(|attr| attr.path.is_ident("inherit_doc")))
        .map(syn::Attribute::clone)
}