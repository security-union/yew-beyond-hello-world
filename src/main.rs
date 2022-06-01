mod msg_ctx;

use js_sys::Array;
use js_sys::Boolean;
use msg_ctx::{MessageContext, MessageProvider};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::*;
use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html!(
        <MessageProvider>
            <VideoReader/>
            <VideoRenderer/>
        </MessageProvider>
    )
}

#[function_component(VideoReader)]
fn video_reader() -> Html {
    use_effect_with_deps(
        move |_| {
            let navigator = window().unwrap().navigator();
            let media_devices = navigator.media_devices().unwrap();

            wasm_bindgen_futures::spawn_local(async move {
                let video_element = window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("webcam")
                    .unwrap()
                    .unchecked_into::<HtmlVideoElement>();
                let mut constraints = MediaStreamConstraints::new();
                constraints.video(&Boolean::from(true));
                let devices_future = media_devices
                    .get_user_media_with_constraints(&constraints)
                    .unwrap();
                let device = JsFuture::from(devices_future)
                    .await
                    .unwrap()
                    .unchecked_into::<MediaStream>();
                console::log_1(&device);
                console::log_1(&video_element);
                video_element.set_src_object(Some(&device));
                let video_tracks = device
                    .get_video_tracks()
                    .find(&mut |_: JsValue, _: u32, _: Array| true);
            });
            || ()
        },
        (),
    );

    let msg_ctx = use_context::<MessageContext>().unwrap();
    let onclick = Callback::from(move |_| msg_ctx.dispatch("Message Received.".to_string()));

    html!(
        <div>
            {"video reader"}
            <video autoplay={true} id="webcam"></video>
            <button {onclick}>
                {"PRESS ME"}
            </button>
        </div>
    )
}

#[function_component(VideoRenderer)]
fn video_renderer() -> Html {
    let msg_ctx = use_context::<MessageContext>().unwrap();
    let message = msg_ctx.inner.to_owned();
    html!(
        <div>
            {"video renderer"}
             <h1>{ message }</h1>
        </div>
    )
}

fn main() {
    yew::start_app::<App>();
}
