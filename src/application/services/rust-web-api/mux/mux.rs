use super::versions::version_one::users;
use axum::routing::{get, post};
use axum::{middleware, Json};
use rust_starter_pack::business::{self, web};
use rust_starter_pack::dependency::logger::logger;
use rust_starter_pack::dependency::server::server::{self, Axum};
use sqlx::postgres;
use std::sync::Arc;

// Mux acts as the multiplexer in order to configure and create our services that acts as the main layer
// For our business logic.

// This is where we provide all our packages, and options to prepare our web handler with the relevant
// features they require to perform business operations.
pub struct MuxConfig<'a> {
    pub web_address: String,
    pub web_port: u16,
    pub debug_address: String,
    pub debug_port: u16,
    // Custom Packages
    pub logger: &'a logger::Logger,
    pub db: postgres::PgPool,
}

// fn new_mux() creates two isolated web services, a debug service, and web service.
// Web service acts as the main service that handles incoming requests, and processes them.
// Debug service acts as the debug server that will contain metrics, and alerting.
pub fn new_mux(config: MuxConfig) -> Result<(Axum, Axum), axum::Error> {
    // Here we add our routes based on version (prefixed)
    let mut v1_axum = initialise_v1_web_routing(&config);
    let debug_axum = initialise_debug_routing(&config);

    // TODO - App level middlewares added in order to wrap over all routes.
    v1_axum.router = v1_axum
        .router
        .route_layer(middleware::from_fn(web::middleware::logging::logging));

    Ok((v1_axum, debug_axum))
}

// fn initialise_debug_routing creates our debug routes, for now, this just contains a root path that pings itself.
// This initial route will help in understanding if the debug service is experiencing any down time.
// But this service can also provide liveness, and readiness checks for our main web server.
fn initialise_debug_routing(config: &MuxConfig) -> Axum {
    let debug_router = axum::Router::new();
    let debug_router = debug_router // We provide a base route to ping.
        .route(
            "/",
            get(|| async {
                let message = "ping successful";
                println!("{}", message);
                Json(message)
            }),
        );
    server::new(server::Config {
        web_address: config.debug_address.clone(),
        port: config.debug_port,
        router: debug_router,
        tracer: String::from(""),
    })
}

// fn initialise_v1_web_routing creates our main web service that contains routes that handle our core business logic.
// Each routing group has its own context that contains any configs and core packages required to perform operations.
// This flow helps to segregate our code and to make sure that ownership is brought down the stack in a consistent
// manner.
fn initialise_v1_web_routing(config: &MuxConfig) -> Axum {
    // Create user handler that will acts as the context for users routes.
    let user_context = users::UserContext {
        version: String::from("v1"),
        user_core: business::core::user::user::new_core(&config.logger, &config.db),
    };

    // Build our router for users.
    let user_router = axum::Router::new()
        // GET ( /v1/users )
        .route("/v1/users", get(users::v1_get_users))
        // GET ( /v1/users/:id )
        .route("/v1/users/:id", get(users::v1_get_user_by_id))
        // POST ( /v1/users )
        .route("/users", post(users::v1_post_user))
        // Create context for users using Arc.
        .with_state(Arc::new(user_context));

    // Here we lastly create our new server, and return to main for it to block the application
    // As stated before, this will be in a seperate thread so we can have multiple senders potentially
    // gracefully shut down the application.
    server::new(server::Config {
        web_address: config.web_address.clone(),
        port: config.web_port,
        // Here we merge our routers that contain different context state, and middlewares.
        router: axum::Router::new().merge(user_router),
        tracer: String::from(""),
    })
}