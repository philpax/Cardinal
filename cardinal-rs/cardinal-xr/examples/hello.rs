use stardust_xr_fusion::{
    client::Client,
    drawable::{Text, TextStyle},
    root::{RootAspect, RootEvent},
    spatial::{Spatial, Transform},
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    eprintln!("hello: connecting...");
    let mut client = Client::connect().await.expect("failed to connect");
    eprintln!("hello: connected!");

    let root = Spatial::create(client.get_root(), Transform::identity()).unwrap();

    let _text = Text::create(
        &root,
        Transform::from_translation([0.0_f32, 0.0, -0.5]),
        "Hello from Cardinal XR!",
        TextStyle {
            character_height: 0.05,
            ..Default::default()
        },
    )
    .unwrap();
    eprintln!("hello: created text, entering event loop");

    client
        .sync_event_loop(|client, _flow| {
            while let Some(event) = client.get_root().recv_root_event() {
                match event {
                    RootEvent::Ping { response } => response.send_ok(()),
                    RootEvent::SaveState { response } => {
                        response.send_ok(stardust_xr_fusion::root::ClientState::default())
                    }
                    _ => (),
                }
            }
        })
        .await
        .unwrap();
}
