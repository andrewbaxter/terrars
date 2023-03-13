use poem::{
    get,
    handler,
    listener::TcpListener,
    Route,
    Server,
};

#[handler]
fn hello() -> String {
    format!("helloophole")
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Route::new().at("/", get(hello));
    Server::new(TcpListener::bind("0.0.0.0:80")).run(app).await
}
