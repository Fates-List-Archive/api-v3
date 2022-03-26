// Endpoints for bot list stats

use crate::models;
use actix_web::{get, web, HttpRequest, HttpResponse};
use procfs;

#[get("/stats")]
async fn get_botlist_stats(req: HttpRequest) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    // If call to procfs panics, we want to error out here anyways
    let uptime = procfs::Uptime::new().unwrap();
    let memory = procfs::Meminfo::new().unwrap();

    HttpResponse::Ok().json(models::ListStats {
        total_bots: data.database.get_bot_count().await,
        total_users: data.database.get_user_count().await,
        total_servers: data.database.get_server_count().await,
        bots: data.database.get_all_bots().await,
        servers: data.database.get_all_servers().await,
        uptime: uptime.uptime,
        cpu_idle: uptime.idle,
        mem_total: memory.mem_total,
        mem_available: memory.mem_available.unwrap_or_default(),
        mem_free: memory.mem_free,
        swap_total: memory.swap_total,
        swap_free: memory.swap_free,
        mem_dirty: memory.dirty,
        mem_active: memory.active,
        mem_inactive: memory.inactive,
        mem_buffers: memory.buffers,
        mem_committed: memory.committed_as,
    })
}
