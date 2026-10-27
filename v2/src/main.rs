#[tokio::main(worker_threads = 2)]
async fn main() {
    if tezbar_extension_sdk::serve(image_to_ascii_native::Adapter::default())
        .await
        .is_err()
    {
        eprintln!("Extension protocol stopped");
        std::process::exit(1);
    }
}
