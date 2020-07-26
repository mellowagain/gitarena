#![macro_escape]

use proc_macro::TokenStream;

#[proc_macro]
pub fn generate_bail(item: TokenStream) -> TokenStream {
    format!("macro_rules! bail {{\
        ( $( $x:expr ),* ) => {{\
            $(\
                match $x {{\
                    Ok(x) => x,\
                    Err(error) => {{\
                        error!(\"Error occurred while processing request: {{}}\", error);\
                        return HttpResponse::InternalServerError().json({}).await\
                    }}\
                }}\
            )*\
        }};\
    }}", item).parse().unwrap()
}
