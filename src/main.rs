use wgpucraft::launcher::run;
use tracy_client::Client;


fn main() {
    let _client = Client::start(); // Inicia el cliente de Tracy

    run();
}
     