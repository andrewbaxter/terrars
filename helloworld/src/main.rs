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
    Server::new(TcpListener::bind("127.0.0.1:80")).run(app).await
}
