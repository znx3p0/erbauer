
use proc_macro::TokenStream;
use derive_syn_parse::Parse;
use quote::{ToTokens, format_ident, quote};
use syn::{Block, FieldsNamed, Ident, Token, Type, TypeTuple, parse::Parse};
use syn::parse_macro_input;

#[derive(Parse, Clone)]
struct Task {
    _task: Ident, // task
    struct_ty: StructType,
    types: TypeTuple,
    _arrow: Token!(=>),
    block: Block
}

#[derive(Clone)]
enum StructType {
    UnitStruct(Ident),
    ExprStruct(Ident, FieldsNamed),
}
impl Parse for StructType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<Ident>()?;
        match input.parse::<FieldsNamed>() {
            Ok(fields) => Ok(StructType::ExprStruct(ident, fields)),
            Err(_) => Ok(StructType::UnitStruct(ident)),
        }
    }
}
enum EmptyStructType {
    Unit, Fielded
}

fn task_to_stream(task: Task) -> TokenStream {
    let mut tuple = task.types;
    let block = task.block;

    tuple.elems.iter_mut().map(|f| {
        let ty = syn::parse::<Type>(quote!(&'static #f).to_token_stream().into()).unwrap();
        *f = ty;
    }).for_each(drop);


    let (strct, ident, stt) = match task.struct_ty {
        StructType::UnitStruct(ident) => {
            (quote!( struct #ident; ), ident, EmptyStructType::Unit)
        },
        StructType::ExprStruct(ident, fields) => {
            (quote!( struct #ident #fields ), ident, EmptyStructType::Fielded)
        },
    };
    let ret1 = match stt {
        EmptyStructType::Unit => (quote!( #ident )),
        EmptyStructType::Fielded => (quote!()),
    };
    let static_ident = format_ident!("{}_CACHE", ident);
    (quote!{
        #strct
        impl ::erbauer::Task for #ident {
            type Dependencies = #tuple;
            fn __run(task: Self::Dependencies) -> &'static Self {
                #static_ident.get_or_init(|| {
                    #block
                    #ret1
                })
            }
        }
        #[allow(non_upper_case_globals)]
        static #static_ident: ::erbauer::OnceCell<#ident> = ::erbauer::OnceCell::new();
        impl Default for &'static #ident {
            fn default() -> Self {
                <#ident as ::erbauer::Task>::run()
            }
        }
    }).into()
}

#[derive(Clone)]
struct Tasks {
    tasks: Vec<Task>
}
impl Parse for Tasks {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut tasks = vec![];
        loop {
            match input.parse::<Task>() {
                Ok(task) => tasks.push(task),
                Err(_) => break,
            };
        }
        Ok(Tasks {
            tasks
        })
    }
}

#[proc_macro]
pub fn tasks(item: TokenStream) -> TokenStream {
    let task = parse_macro_input!(item as Tasks);
    let mut streams = vec![];
    for task in task.tasks {
        streams.push(task_to_stream(task));
    }
    let mut t = TokenStream::new();
    for stream in streams {
        t.extend(stream);
    }
    t
}

#[proc_macro]
pub fn erbauer(item: TokenStream) -> TokenStream {
    let task = parse_macro_input!(item as Tasks);
    let mut streams = vec![];
    let mut names = vec![];
    for task in task.tasks {
        streams.push(task_to_stream(task.clone()));
        let name = match task.struct_ty {
            StructType::UnitStruct(name) => name,
            StructType::ExprStruct(name, _) => name,
        };
        names.push(name);
    }

    let mut t = TokenStream::new();
    for stream in streams {
        t.extend(stream);
    }
    let mut arrows = vec![];
    let mut contains_main = false;
    for name in names {
        let lower = name.to_string().to_lowercase();
        if lower.as_str() == "main" {
            contains_main = true;
        }
        arrows.push(quote! (
            #lower => { #name::run(); },
        ));
    }
    if !contains_main {
        panic!("Main task not found")
    }

    let mut ts = TokenStream::from(quote!{
        fn main() {
            let org = std::env::args().skip(1).nth(0).unwrap_or("main".into());
            match org.to_lowercase().as_str() {
                #(#arrows)*
                _ => {
                    println!("Command {:?} not recognized", org);
                    std::process::exit(1);
                }
            };
        }
    });
    ts.extend(t);
    ts.into()
}


