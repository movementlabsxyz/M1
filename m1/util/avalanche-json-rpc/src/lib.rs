extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStruct, LitStr};

#[proc_macro_attribute]
pub fn avalanche_chain_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as ItemStruct);

    // Extract the RPC trait from args
    let rpc_trait: LitStr = syn::parse(args).expect("Expected a trait name");
    let rpc_trait = rpc_trait.value();

    // Extract the struct name
    let struct_name = &input.ident;

    // Generate the desired code
    let expanded = quote! {
        pub struct #struct_name<T> {
            pub handler: jsonrpc_core::IoHandler,
            _marker: std::marker::PhantomData<T>,
        }

        #[tonic::async_trait]
        impl<T> avalanche_types::subnet::rpc::http::handle::Handle for #struct_name<T>
        where
            T: #rpc_trait + Send + Sync + Clone + 'static,
        {
            async fn request(
                &self,
                req: &bytes::Bytes,
                _headers: &[avalanche_types::proto::http::Element],
            ) -> std::io::Result<(bytes::Bytes, Vec<avalanche_types::proto::http::Element>)> {
                match self.handler.handle_request(&de_request(req)?).await {
                    Some(resp) => Ok((bytes::Bytes::from(resp), Vec::new())),
                    None => Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "failed to handle request",
                    )),
                }
            }
        }

        impl<T: #rpc_trait> #struct_name<T> {
            pub fn new(service: T) -> Self {
                let mut handler = jsonrpc_core::IoHandler::new();
                handler.extend_with(#rpc_trait::to_delegate(service));
                Self {
                    handler,
                    _marker: std::marker::PhantomData,
                }
            }
        }
    };

    expanded.into()
}
