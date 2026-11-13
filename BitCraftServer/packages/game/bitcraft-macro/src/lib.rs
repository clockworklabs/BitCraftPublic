#![crate_type = "proc-macro"]
extern crate proc_macro;

use proc_macro::*;
use quote::{quote, ToTokens};

// This derive is actually a no-op, we need the helper attribute for code generation
#[proc_macro_derive(Operations, attributes(operations))]
pub fn derive_commit(_: TokenStream) -> TokenStream {
    TokenStream::new()
}

#[proc_macro_attribute]
pub fn static_data_staging_table(attr: TokenStream, input: TokenStream) -> TokenStream {
    //Add spacetimedb::table attribute for staged table

    let attr_ident = match syn::parse::<syn::Ident>(attr.clone()) {
        Ok(r) => r,
        _ => return input,
    };
    let name = attr.to_string();
    let table_name = syn::Ident::new(format!("staged_{name}").as_str(), attr_ident.span());

    let mut ast: syn::ItemStruct = match syn::parse(input.clone()) {
        Ok(val) => val,
        Err(_) => return input,
    };
    ast.attrs.insert(
        0,
        syn::parse_quote! {
            #[spacetimedb::table(accessor = #table_name)]
        },
    );
    TokenStream::from(quote!(#ast))
}

#[proc_macro_attribute]
pub fn custom_inter_module_insert(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Just pass through the item unchanged
    item
}
// Attribute does additional code generation
#[proc_macro_attribute]
pub fn shared_table(_args: TokenStream, input: TokenStream) -> TokenStream {
    //let err = compile_error("Expected usage: #[shared_table] OR #[shared_table(public_region | public_global)]");
    //let args_parsed: syn::ExprAssign = match syn::parse(args) {
    //    Ok(val) => val,
    //    Err(_) => return err,
    //};

    let ast: syn::DeriveInput = match syn::parse(input.clone()) {
        Ok(val) => val,
        Err(_) => return input,
    };

    //Find primary_key name (need for deletes)
    let str = input.to_string();
    let ind = match str.find("primary_key") {
        Some(val) => val,
        None => return compile_error("Missing `primary_key` attribute"),
    };
    let ind2 = str[ind..].find(':').unwrap();
    let pk = str[ind..(ind + ind2)].trim();
    let ind = match pk.rfind("pub") {
        Some(val) => val,
        None => return compile_error("Primary key field must be public"),
    };
    let pk = pk[(ind + 4)..].trim();
    let pk = syn::Ident::new(pk, ast.ident.span());

    let name = &ast.ident;
    let name_snake = camel_to_snake(&name.to_string());
    let name_snake = syn::Ident::new(name_snake.as_str(), name.span());
    let op_type = syn::Ident::new(format!("{}Op", name).as_str(), name.span());
    let gen = quote! {
        use spacetimedb::*;
        impl #name {
            //pub fn insert_local(ctx: &ReducerContext, val: #name) {
            //    ctx.db.#name_snake().insert(val);
            //}
            pub fn insert_shared(ctx: &ReducerContext, val: #name, destination: crate::inter_module::InterModuleDestination) {
                ctx.db.#name_snake().insert(val.clone());

                match destination {
                    crate::inter_module::InterModuleDestination::Global |
                    crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions => {
                        crate::inter_module::add_global_table_update(|u| {
                            if u.#name_snake.is_none() {
                                u.#name_snake = Some(Vec::new());
                            }
                            u.#name_snake.as_mut().unwrap().push(crate::inter_module::_autogen::#op_type::Insert(val.clone()));
                        });
                    }
                    crate::inter_module::InterModuleDestination::AllOtherRegions => {}
                    crate::inter_module::InterModuleDestination::Region(_) => panic!("Table updates cannot be sent to specific regions"),
                    _ => panic!("Unhandled case"),
                }

                match destination {
                    crate::inter_module::InterModuleDestination::AllOtherRegions |
                    crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions => {
                        crate::inter_module::add_region_table_update(|u| {
                            if u.#name_snake.is_none() {
                                u.#name_snake = Some(Vec::new());
                            }
                            u.#name_snake.as_mut().unwrap().push(crate::inter_module::_autogen::#op_type::Insert(val));
                        });
                    }
                    _ => { }
                }
            }

            //pub fn delete_local(ctx: &ReducerContext, val: #name) {
            //    ctx.db.#name_snake().delete(val);
            //}
            pub fn delete_shared(ctx: &ReducerContext, val: #name, destination: crate::inter_module::InterModuleDestination) {
                ctx.db.#name_snake().#pk().delete(val.#pk);

                match destination {
                    crate::inter_module::InterModuleDestination::Global |
                    crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions => {
                        crate::inter_module::add_global_table_update(|u| {
                            if u.#name_snake.is_none() {
                                u.#name_snake = Some(Vec::new());
                            }
                            u.#name_snake.as_mut().unwrap().push(crate::inter_module::_autogen::#op_type::Delete(val.clone()));
                        });
                    }
                    crate::inter_module::InterModuleDestination::AllOtherRegions => {}
                    crate::inter_module::InterModuleDestination::Region(_) => panic!("Table updates cannot be sent to specific regions"),
                    _ => panic!("Unhandled case"),
                }

                match destination {
                    crate::inter_module::InterModuleDestination::AllOtherRegions |
                    crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions => {
                        crate::inter_module::add_region_table_update(|u| {
                            if u.#name_snake.is_none() {
                                u.#name_snake = Some(Vec::new());
                            }
                            u.#name_snake.as_mut().unwrap().push(crate::inter_module::_autogen::#op_type::Delete(val));
                        });
                    }
                    _ => { }
                }
            }

            pub fn update_shared(ctx: &ReducerContext, val: #name, destination: crate::inter_module::InterModuleDestination) {
                Self::delete_shared(ctx, val.clone(), destination);
                Self::insert_shared(ctx, val, destination);
            }
        }
    };

    let mut gen_tokens = TokenStream::from(gen);
    gen_tokens.extend(input);
    gen_tokens
}

//Create a table update accumulator before any reducer code runs
#[proc_macro_attribute]
pub fn shared_table_reducer(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut ast: syn::ItemFn = match syn::parse(input.clone()) {
        Ok(val) => val,
        Err(_) => return input,
    };

    let gen_start = quote! {
        {
            let __shared_transaction_accumulator = crate::inter_module::SharedTransactionAccumulator { ctx: ctx };
            __shared_transaction_accumulator.begin_shared_transaction();
        }
    };
    let ast_start: syn::Block = syn::parse2(gen_start).unwrap();
    ast.block.stmts.insert(0, ast_start.stmts[0].clone());
    ast.block.stmts.insert(1, ast_start.stmts[1].clone());
    proc_macro::TokenStream::from(ast.into_token_stream())
}

#[proc_macro_attribute]
pub fn feature_gate(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut ast: syn::ItemFn = match syn::parse(input.clone()) {
        Ok(val) => val,
        Err(_) => return input,
    };

    let category = match parse_feature_gate_args(args) {
        Ok(value) => value,
        Err(err) => return err,
    };

    if !return_type_is_result_string(&ast.sig.output) {
        return compile_error("`#[feature_gate]` can only be used on reducers returning `Result<_, String>`");
    }

    let ctx_ident = match find_reducer_context_ident(&ast.sig.inputs) {
        Some(ident) => ident,
        None => return compile_error("`#[feature_gate]` requires a named `ReducerContext` parameter"),
    };

    let reducer_name = syn::LitStr::new(ast.sig.ident.to_string().as_str(), ast.sig.ident.span());
    let category_gate = if let Some(category_name) = category {
        quote! {
            let __feature_gate_category_key = format!("category:{}", #category_name);
            if crate::messages::generic::gated_features::gated_features(&#ctx_ident.db)
                .feature()
                .find(&__feature_gate_category_key)
                .is_some()
            {
                return Err("This functionality is currently disabled".into());
            }
        }
    } else {
        quote! {}
    };

    let gen_start = quote! {{
        if !crate::game::handlers::authentication::has_role(
            #ctx_ident,
            &#ctx_ident.sender(),
            crate::messages::authentication::Role::Gm,
        ) {
            let __feature_gate_reducer_key = format!("reducer:{}", #reducer_name);
            if crate::messages::generic::gated_features::gated_features(&#ctx_ident.db)
                .feature()
                .find(&__feature_gate_reducer_key)
                .is_some()
            {
                return Err("This functionality is currently disabled".into());
            }
            #category_gate
        }
    }};

    let ast_start: syn::Block = syn::parse2(gen_start).unwrap();
    for (idx, stmt) in ast_start.stmts.into_iter().enumerate() {
        ast.block.stmts.insert(idx, stmt);
    }

    proc_macro::TokenStream::from(ast.into_token_stream())
}

fn camel_to_snake(str: &str) -> String {
    let mut output = String::new();
    for c in str.chars() {
        if c.is_uppercase() {
            if String::is_empty(&output) {
                output.push_str(c.to_ascii_lowercase().to_string().as_str());
            } else {
                output.push_str(format!("_{}", c.to_ascii_lowercase()).as_str());
            }
        } else {
            output.push_str(c.to_string().as_str());
        }
    }
    output
}

//Create a timer table for client events
#[proc_macro_attribute]
pub fn event_table(args: TokenStream, input: TokenStream) -> TokenStream {
    //Parse parameters
    let err = compile_error("Expected usage: #[event_table(name=table_name)]");
    let args_parsed: syn::ExprAssign = match syn::parse(args) {
        Ok(val) => val,
        Err(_) => return err,
    };

    let mut is_valid = false;
    let mut table_name = String::new();
    if let syn::Expr::Path(left) = *args_parsed.left {
        if let syn::Expr::Path(right) = *args_parsed.right {
            if left.path.segments.len() == 1 && left.path.segments[0].ident == "name" && right.path.segments.len() == 1 {
                is_valid = true;
                table_name = right.path.segments[0].ident.to_string();
            }
        }
    }
    if !is_valid {
        return err;
    }

    //Parse input
    let ast: syn::DeriveInput = match syn::parse(input.clone()) {
        Ok(val) => val,
        Err(_) => return input,
    };
    let name = &ast.ident;
    let span = name.span();
    let table_name = syn::Ident::new(table_name.as_str(), span);
    let reducer_name = syn::Ident::new(format!("{}_reducer", table_name).as_str(), span);

    //Generate timer table fields
    let gen_fields = quote! {
        struct Tmp {
            #[primary_key]
            #[auto_inc]
            pub scheduled_id: u64,
            pub scheduled_at: spacetimedb::ScheduleAt,
        }
    };
    //Generate struct, impl and reducer
    let gen = quote! {
        #[spacetimedb::table(accessor = #table_name, public, scheduled(#reducer_name, at = scheduled_at))]
        pub struct #name {
            //Fields are added later on
        }

        impl #name {
            pub fn new_event(ctx: &spacetimedb::ReducerContext) {
                let val = #name {
                    scheduled_id: 0,
                    scheduled_at: ctx.timestamp.into(),
                };
                //We can't simply do `ctx.db.#name_snake().insert(val)` as that requires importing a trait
                let table = #table_name::#table_name(&ctx.db);
                spacetimedb::Table::insert(table, val);
            }
        }

        #[spacetimedb::reducer]
        fn #reducer_name(_ctx: &spacetimedb::ReducerContext, _timer: #name) { }
    };

    let mut ast_gen: syn::File = syn::parse2(gen).unwrap();
    let ast_gen_fields: syn::ItemStruct = syn::parse2(gen_fields).unwrap();
    if let syn::Data::Struct(src_struct) = &ast.data {
        //Combine generated code for struct
        if let syn::Item::Struct(s) = &mut ast_gen.items[0] {
            //Copy original fields
            s.fields = src_struct.fields.clone();

            //Add timer table fields
            if let syn::Fields::Named(fields) = &mut s.fields {
                if let syn::Fields::Named(extra_fields) = &ast_gen_fields.fields {
                    for i in 0..extra_fields.named.len() {
                        fields.named.insert(i, extra_fields.named[i].clone());
                    }
                }
            }
        }

        //Add struct parameters to new_event
        if let syn::Item::Impl(imp) = &mut ast_gen.items[1] {
            if let syn::ImplItem::Fn(f) = &mut imp.items[0] {
                //Add arguments to fn def
                for field in &src_struct.fields {
                    let field = field.clone();
                    f.sig.inputs.push(syn::FnArg::Typed(syn::PatType {
                        attrs: vec![],
                        colon_token: syn::token::Colon { spans: [span] },
                        pat: Box::new(syn::Pat::Ident(syn::PatIdent {
                            attrs: vec![],
                            by_ref: None,
                            mutability: None,
                            ident: field.ident.unwrap(),
                            subpat: None,
                        })),
                        ty: Box::new(field.ty),
                    }));
                }

                //Add fields to table row
                if let syn::Stmt::Local(loc) = &mut f.block.stmts[0] {
                    if let Some(init) = &mut loc.init {
                        if let syn::Expr::Struct(s) = &mut *init.expr {
                            for field in &src_struct.fields {
                                let field = field.clone();
                                let mut segments = syn::punctuated::Punctuated::new();
                                segments.push(syn::PathSegment {
                                    ident: field.ident.clone().unwrap(),
                                    arguments: syn::PathArguments::None,
                                });
                                s.fields.push(syn::FieldValue {
                                    attrs: vec![],
                                    member: syn::Member::Named(field.ident.clone().unwrap()),
                                    colon_token: None,
                                    expr: syn::Expr::Path(syn::ExprPath {
                                        attrs: vec![],
                                        qself: None,
                                        path: syn::Path {
                                            leading_colon: None,
                                            segments,
                                        },
                                    }),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    proc_macro::TokenStream::from(ast_gen.into_token_stream())
}

fn compile_error(err: &str) -> TokenStream {
    proc_macro::TokenStream::from(syn::Error::new(proc_macro2::Span::from(Span::call_site()), err.to_string()).to_compile_error())
}

fn parse_feature_gate_args(args: TokenStream) -> Result<Option<syn::LitStr>, TokenStream> {
    if args.is_empty() {
        return Ok(None);
    }

    match syn::parse::<syn::LitStr>(args) {
        Ok(v) => Ok(Some(v)),
        Err(_) => Err(compile_error(
            "Expected usage: #[feature_gate] or #[feature_gate(\"category\")]",
        )),
    }
}

fn find_reducer_context_ident(
    inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
) -> Option<syn::Ident> {
    for arg in inputs {
        if let syn::FnArg::Typed(pat_type) = arg {
            let pat_ident = match &*pat_type.pat {
                syn::Pat::Ident(pat_ident) => pat_ident.ident.clone(),
                _ => continue,
            };
            if type_is_reducer_context(&pat_type.ty) {
                return Some(pat_ident);
            }
        }
    }
    None
}

fn type_is_reducer_context(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Reference(ref_type) => type_is_reducer_context(&ref_type.elem),
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "ReducerContext"),
        _ => false,
    }
}

fn return_type_is_result_string(output: &syn::ReturnType) -> bool {
    let return_ty = match output {
        syn::ReturnType::Type(_, ty) => &**ty,
        syn::ReturnType::Default => return false,
    };

    let return_ty_path = match return_ty {
        syn::Type::Path(type_path) => &type_path.path,
        _ => return false,
    };

    let result_segment = match return_ty_path.segments.last() {
        Some(segment) if segment.ident == "Result" => segment,
        _ => return false,
    };

    let args = match &result_segment.arguments {
        syn::PathArguments::AngleBracketed(args) => &args.args,
        _ => return false,
    };

    if args.len() != 2 {
        return false;
    }

    let error_ty = match args.iter().nth(1) {
        Some(syn::GenericArgument::Type(ty)) => ty,
        _ => return false,
    };

    type_is_string(error_ty)
}

fn type_is_string(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "String"),
        _ => false,
    }
}
