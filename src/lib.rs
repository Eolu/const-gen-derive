use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn;

#[proc_macro_derive(CompileConst)]
pub fn const_gen_derive(input: TokenStream) -> TokenStream 
{
    impl_macro(&syn::parse(input).unwrap())
}

fn impl_macro(ast: &syn::DeriveInput) -> TokenStream 
{
    let name = &ast.ident;
    let generics = &ast.generics;
    let val_impl = match &ast.data
    {
        syn::Data::Struct(d) => struct_field_handler(name, &d.fields),
        syn::Data::Enum(f) => 
        {
            let arms: Vec<proc_macro2::TokenStream> = f.variants
                .iter()
                .map(|v| 
                {
                    let var_ident = &v.ident;
                    let constructor = enum_field_handler(name, &v.ident, &v.fields);
                    match &v.fields
                    {
                        syn::Fields::Named(f) => 
                        {
                            let new_idents: Vec<&Option<syn::Ident>> = f.named.iter().map(|f| &f.ident).collect();
                            quote!(Self::#var_ident {#(#new_idents, )*} => #constructor)
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
                            quote!(Self::#var_ident (#(#new_idents, )*) => #constructor)
                        }
                        syn::Fields::Unit => quote!(Self::#var_ident => #constructor)
                    }
                })
                .collect();
            quote!
            {
                format!("{}", match self
                {
                    #( #arms, )*
                })
            }
        }
        syn::Data::Union(f) => 
        {
            let ident: &Option<syn::Ident> = f.fields.named.iter().map(|field|&field.ident).next().unwrap();
            quote!
            {
                format!
                (
                    "{} {{ {}: {}}}", 
                    stringify!(#name), 
                    stringify!(#ident), 
                    self.#ident.const_val()
                )
            }
        }
    };
    let gen = quote! 
    {
        impl const_gen::CompileConst for #name #generics
        {
            const CONST_TYPE: const_gen::ConstType = const_gen::ConstType::Dependant;

            fn const_type(&self) -> String 
            {
                String::from(stringify!(#name))
            }

            fn const_val(&self) -> String 
            {
                #val_impl
            }
        }
    };
    gen.into()
}

fn struct_field_handler(name: &syn::Ident, fields: &syn::Fields) -> proc_macro2::TokenStream
{
    match fields
    {
        syn::Fields::Named(f) => 
        {
            let idents: Vec<&Option<syn::Ident>> = f.named.iter().map(|field|&field.ident).collect();
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

fn enum_field_handler(name: &syn::Ident, var_name: &syn::Ident, fields: &syn::Fields) -> proc_macro2::TokenStream
{
    match fields
    {
        syn::Fields::Named(f) => 
        {
            let idents: Vec<&Option<syn::Ident>> = f.named.iter().map(|field|&field.ident).collect();
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
    } 
}
