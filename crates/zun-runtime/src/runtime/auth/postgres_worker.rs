use std::sync::{Arc, Mutex};
use std::thread;

use postgres::{Client, NoTls};

use crate::error::Error;
use crate::runtime::sql_registry::PostgresClientHandle;

pub(crate) fn connect_client(
    url: &str,
    error_prefix: &'static str,
) -> Result<Arc<Mutex<PostgresClientHandle>>, Error> {
    let url = url.to_string();
    let client = thread::spawn(move || {
        Client::connect(url.as_str(), NoTls)
            .map(PostgresClientHandle::new)
            .map_err(|err| Error::Interpreter(format!("{error_prefix}: {err}")))
    })
    .join()
    .map_err(|_| {
        Error::Interpreter(format!("{error_prefix}: postgres connect worker panicked"))
    })??;

    Ok(Arc::new(Mutex::new(client)))
}

pub(crate) fn with_client<T>(
    client: &Arc<Mutex<PostgresClientHandle>>,
    worker_name: &'static str,
    f: impl FnOnce(&mut Client) -> Result<T, Error> + Send + 'static,
) -> Result<T, Error>
where
    T: Send + 'static,
{
    let client = Arc::clone(client);
    thread::spawn(move || {
        let mut client = client.lock().map_err(|_| {
            Error::Interpreter(format!("{worker_name}: postgres client lock was poisoned"))
        })?;
        f(client.client_mut()?)
    })
    .join()
    .map_err(|_| Error::Interpreter(format!("{worker_name}: postgres worker panicked")))?
}
