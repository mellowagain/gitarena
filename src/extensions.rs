use actix_web::HttpRequest;

pub(crate) fn get_user_agent(request: &HttpRequest) -> Option<&str> {
    request.headers().get("user-agent")?.to_str().ok()
}
