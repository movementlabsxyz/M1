use std::io::{Error, ErrorKind};

use avalanche_types::{
    self,
    proto::{
        grpcutil::default_server,
        rpcdb::database_server::{Database, DatabaseServer},
    },
};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Channel;

pub async fn create_conn() -> Channel {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    Channel::builder(format!("http://{}", addr).parse().unwrap())
        .connect()
        .await
        .unwrap()
}

pub async fn serve_test_database<D: Database + 'static>(
    database: D,
    listener: TcpListener,
) -> std::io::Result<()>
where
    D: Database,
{
    default_server()
        .add_service(DatabaseServer::new(database))
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to serve test database service: {}", e),
            )
        })
}
