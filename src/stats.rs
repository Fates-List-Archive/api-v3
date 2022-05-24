// Endpoints for bot list stats

use crate::models;
use actix_web::{get, web, HttpRequest, HttpResponse};

#[get("/stats")]
async fn get_botlist_stats(req: HttpRequest) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    // If call to procfs panics, we want to error out here anyways
    let uptime = procfs::Uptime::new().unwrap();

    HttpResponse::Ok().json(models::ListStats {
        total_bots: data.database.get_bot_count().await,
        total_users: data.database.get_user_count().await,
        total_servers: data.database.get_server_count().await,
        bots: data.database.get_all_bots().await,
        servers: data.database.get_all_servers().await,
        uptime: uptime.uptime,
    })
}
