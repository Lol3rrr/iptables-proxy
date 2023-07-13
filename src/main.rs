use clap::Parser;
use iptables_proxy::ForwardingRoute;
use tracing_subscriber::{layer::Filter, prelude::__tracing_subscriber_SubscriberExt, Layer};

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use std::{
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};

use serde::Deserialize;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long = "public-ip")]
    public_ip: IpAddr,
    #[clap(long = "listen-addr", default_value = "127.0.0.1")]
    listen_addr: IpAddr,
    #[clap(long = "listen-port", default_value = "8080")]
    listen_port: u16,
    #[clap(long, short, action)]
    dry_run: bool,
}

struct CrateFilter {}

impl<S> Filter<S> for CrateFilter {
    fn enabled(
        &self,
        meta: &tracing::Metadata<'_>,
        _cx: &tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        meta.target() == "iptables_proxy"
    }
}

struct AppState {
    routes: Mutex<Vec<ForwardingRoute>>,
    dry_run: bool,
    public_ip: String,
}

impl AppState {
    pub fn add(&self, route: ForwardingRoute) -> Option<ForwardingRoute> {
        let mut existing_routes = self.routes.lock().unwrap();

        if let Some((idx, _)) = existing_routes.iter().enumerate().find(|(_, r)| {
            r.public_ip() == route.public_ip() && r.public_port() == route.public_port()
        }) {
            tracing::error!("Route already exists, replacing existing route");
            let old_route = existing_routes.remove(idx);

            existing_routes.push(route.clone());
            drop(existing_routes);

            Some(old_route)
        } else {
            existing_routes.push(route.clone());

            None
        }
    }

    pub fn remove(&self, req: &RemoveRequest) -> Option<ForwardingRoute> {
        let mut existing_routes = self.routes.lock().unwrap();

        match existing_routes
            .iter()
            .enumerate()
            .find(|(_, r)| r.public_ip() == self.public_ip && r.public_port() == req.public_port)
        {
            Some((i, _)) => Some(existing_routes.remove(i)),
            None => None,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = Args::parse();

    let fmt_layer = tracing_subscriber::fmt::Layer::new()
        .with_ansi(cfg!(debug_assertions))
        .with_filter(CrateFilter {});

    let sub = tracing_subscriber::registry().with(fmt_layer);

    tracing::subscriber::set_global_default(sub).unwrap();

    tracing::info!("Listening on {}:{}", args.listen_addr, args.listen_port);
    if args.dry_run {
        tracing::info!("Running in dry run mode");
    }

    let app_state = Arc::new(AppState {
        routes: Mutex::new(Vec::new()),
        dry_run: args.dry_run,
        public_ip: format!("{}", args.public_ip),
    });

    let app = Router::new()
        .route("/create", post(create))
        .route("/remove", post(remove))
        .with_state(app_state);

    let addr = SocketAddr::from((args.listen_addr, args.listen_port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Debug, Deserialize)]
struct CreateRequest {
    public_port: u16,
    inner_port: u16,
    inner_ip: String,
    protocol: String,
}

#[tracing::instrument(skip(state, payload))]
#[axum::debug_handler]
async fn create(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateRequest>,
) -> StatusCode {
    tracing::debug!("Received request to create route: {:?}", payload);

    let route = ForwardingRoute::new(
        (state.public_ip.clone(), payload.public_port),
        (payload.inner_ip.clone(), payload.inner_port),
        payload.protocol.clone(),
    );

    if let Some(old_route) = state.add(route.clone()) {
        tracing::error!("Route already exists, replacing existing route");

        // Deregister the old route from iptables
        for mut cmd in old_route.deregister() {
            tracing::debug!("Running Command: {:?}", cmd);
            if !state.dry_run {
                if let Err(e) = cmd.status().await {
                    tracing::error!("Executing Command: {:?}", e);
                }
            }
        }
    }

    for mut cmd in route.register() {
        tracing::debug!("Running Command: {:?}", cmd);
        if !state.dry_run {
            if let Err(e) = cmd.status().await {
                tracing::error!("Executing Command: {:?}", e);
            }
        }
    }

    tracing::debug!("Created route");

    StatusCode::OK
}

#[derive(Debug, Deserialize)]
struct RemoveRequest {
    public_port: u16,
}

#[tracing::instrument(skip(state, payload))]
async fn remove(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RemoveRequest>,
) -> StatusCode {
    tracing::debug!("Received request to remove route: {:?}", payload);

    let route = state.remove(&payload);

    let route = match route {
        Some(r) => r,
        None => {
            tracing::error!("Tried to remove non existing Route: {:?}", payload);
            return StatusCode::BAD_REQUEST;
        }
    };

    for mut cmd in route.deregister() {
        tracing::debug!("Running Command: {:?}", cmd);
        if !state.dry_run {
            if let Err(e) = cmd.status().await {
                tracing::error!("Executing Command: {:?}", e);
            }
        }
    }

    tracing::debug!("Removed route");

    StatusCode::OK
}
