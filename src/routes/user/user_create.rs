use actix_web::{HttpResponse, post, Responder, web};
use crate::user::User;
use crate::{captcha, PgPoolConnection};
use gitarena_proc_macro::generate_bail;
use log::error;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgQueryAs;
use sqlx::{Connection, PgPool, Transaction};

generate_bail!(RegisterJsonResponse {
                   success: false,
                   id: None,
                   errors: Some("Internal server error occurred".to_owned())
               });

#[post("/api/user")]
pub(crate) async fn register(body: web::Json<RegisterJsonRequest>, db_pool: web::Data<PgPool>) -> impl Responder {
    let connection = bail!(db_pool.acquire().await);
    let mut transaction: Transaction<PgPoolConnection> = bail!(connection.begin().await);

    let (exists,): (bool,) = bail!(sqlx::query_as("select exists(select 1 from users where username = $1);")
        .bind(&body.username)
        .fetch_one(&mut transaction)
        .await);

    if exists {
        return HttpResponse::Conflict().json(RegisterJsonResponse {
            success: false,
            id: None,
            errors: Some("Username already in use".to_owned())
        }).await;
    }

    let captcha_success = bail!(captcha::verify_captcha(&body.h_captcha_response.to_owned()).await);

    if !captcha_success {
        return HttpResponse::UnprocessableEntity().json(RegisterJsonResponse {
            success: false,
            id: None,
            errors: Some("Captcha verification failed".to_owned())
        }).await;
    }

    let mut user = bail!(User::new(
        body.username.to_owned(), body.email.to_owned(), body.password.to_owned()
    ));
    bail!(user.save(db_pool.get_ref()).await);

    bail!(transaction.commit().await);
    HttpResponse::Ok().json(RegisterJsonResponse {
        success: true,
        id: Some(user.id),
        errors: None
    }).await
}

#[derive(Deserialize)]
pub(crate) struct RegisterJsonRequest {
    username: String,
    email: String,
    password: String,
    h_captcha_response: String
}

#[derive(Serialize)]
struct RegisterJsonResponse {
    success: bool,
    id: Option<i32>,
    errors: Option<String>
}
